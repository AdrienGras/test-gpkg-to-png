mod cli;
mod error;
mod geojson;
mod gpkg;
mod math;
mod render;

use clap::Parser;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::time::Instant;

use cli::Args;
use error::{GpkgError, Result};
use gpkg::{reproject_bbox_to_wgs84, GpkgReader, LayerInfo};
use math::Bbox;
use render::{RenderConfig, Renderer};

/// Entry point of the application.
#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

/// Orchestrates the global processing pipeline.
///
/// 1. Parses CLI arguments and validates configuration.
/// 2. Dispatches to format-specific processor (GPKG or GeoJSON).
async fn run() -> Result<()> {
    let args = Args::parse();
    let config = args.validate()?;

    // Check input file exists
    if !config.input.exists() {
        return Err(GpkgError::FileNotFound(config.input.display().to_string()));
    }

    // Create output directory if needed
    if !config.output_dir.exists() {
        std::fs::create_dir_all(&config.output_dir)?;
    }

    match config.format {
        cli::Format::Gpkg => process_gpkg(config).await?,
        cli::Format::Geojson => {
            todo!("implement process_geojson")
        }
    }

    Ok(())
}

/// Process a GeoPackage file (multi-layer workflow).
async fn process_gpkg(config: cli::Config) -> Result<()> {
    let start_total = Instant::now();

    // Open GeoPackage
    let reader = GpkgReader::open(&config.input).await?;

    // Get layers to process
    let all_layers = reader.list_polygon_layers().await?;

    if all_layers.is_empty() {
        eprintln!("Warning: No polygon layers found in the GeoPackage");
        return Ok(());
    }

    let layers_to_process = match &config.layer {
        Some(name) => {
            let layer = all_layers.iter().find(|l| l.name == *name).ok_or_else(|| {
                let available = all_layers
                    .iter()
                    .map(|l| l.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                GpkgError::LayerNotFound(name.clone(), available)
            })?;
            vec![layer.clone()]
        }
        None => all_layers,
    };

    // Determine bounding box (use provided or auto-detect from layers)
    let bbox = if let Some(bbox) = config.bbox {
        bbox
    } else {
        println!("Auto-detecting bounding box...");
        // Auto-detect from all layers
        let mut union_bbox: Option<(f64, f64, f64, f64)> = None;

        for layer in &layers_to_process {
            if let Some((min_x, min_y, max_x, max_y)) = reader.get_layer_bbox(layer).await? {
                let srs_def = reader.get_srs_definition(layer.srs_id).await?;

                if let Some((lon_min, lat_min, lon_max, lat_max)) =
                    reproject_bbox_to_wgs84(min_x, min_y, max_x, max_y, &srs_def)
                {
                    union_bbox = Some(match union_bbox {
                        None => (lon_min, lat_min, lon_max, lat_max),
                        Some((a, b, c, d)) => (
                            a.min(lon_min),
                            b.min(lat_min),
                            c.max(lon_max),
                            d.max(lat_max),
                        ),
                    });
                }
            }
        }

        let (min_lon, min_lat, max_lon, max_lat) = union_bbox.ok_or_else(|| {
            GpkgError::InvalidBbox("Could not determine bounding box from layers".to_string())
        })?;

        println!(
            "Auto-detected bbox: {},{},{},{}",
            min_lon, min_lat, max_lon, max_lat
        );
        Bbox::new(min_lon, min_lat, max_lon, max_lat)
    };

    // Compute resolution from scale if needed
    let resolution = if let Some(scale) = config.scale {
        let center_lat = (bbox.min_lat + bbox.max_lat) / 2.0;
        let resolution = scale / (111319.0 * center_lat.to_radians().cos());
        println!(
            "Scale: {} m/pixel -> Resolution: {:.10} deg/pixel",
            scale, resolution
        );
        resolution
    } else {
        config.resolution.unwrap()
    };

    println!("Processing {} layer(s)...", layers_to_process.len());

    let multi = MultiProgress::new();
    let main_pb = multi.add(ProgressBar::new(layers_to_process.len() as u64));
    main_pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );

    // Process each layer
    for layer in &layers_to_process {
        main_pb.set_message(format!("Layer: {}", layer.name));

        process_layer(
            &reader,
            layer,
            &bbox,
            resolution,
            &config,
            &multi,
        ).await?;

        main_pb.inc(1);
    }

    main_pb.finish_with_message("All layers processed");

    let duration = start_total.elapsed();
    println!("Total time: {:.2?}", duration);
    Ok(())
}

/// Processes a single GeoPackage layer.
///
/// This involves:
/// 1. Reading and reprojecting geometries to WGS84.
/// 2. Initializing the renderer and rasterizing each MultiPolygon.
/// 3. Saving the final image as a PNG.
async fn process_layer(
    reader: &GpkgReader,
    layer: &LayerInfo,
    bbox: &Bbox,
    resolution: f64,
    config: &cli::Config,
    multi: &MultiProgress,
) -> Result<()> {
    let start_layer = Instant::now();

    // 1. Read and reproject
    let pb = multi.add(ProgressBar::new_spinner());
    pb.set_style(ProgressStyle::default_spinner().template("{spinner:.green} {msg}").unwrap());
    pb.set_message(format!("Reading and reprojecting {}...", layer.name));

    let start_read = Instant::now();
    let geometries = reader.read_geometries_wgs84(layer).await?;
    let duration_read = start_read.elapsed();

    if geometries.is_empty() {
        pb.finish_with_message(format!("  Layer {}: skipped (no geometries)", layer.name));
        return Ok(());
    }

    // 2. Render
    let render_config = RenderConfig {
        bbox: *bbox,
        resolution,
        fill: config.fill,
        stroke: config.stroke,
        stroke_width: config.stroke_width,
    };

    let renderer = Renderer::new(render_config)?;
    let (width, height) = renderer.dimensions();

    pb.set_style(
        ProgressStyle::default_bar()
            .template("    {msg} [{bar:20.yellow/orange}] {pos}/{len} ({percent}%)")
            .unwrap()
            .progress_chars("=>-"),
    );
    pb.set_length(geometries.len() as u64);
    pb.set_message(format!("Rendering {} ({}x{})", layer.name, width, height));

    let start_render = Instant::now();
    // Render all geometries (using the parallelized renderer internally)
    for (i, geom) in geometries.iter().enumerate() {
        renderer.render_multipolygon(geom);
        pb.set_position((i + 1) as u64);
    }
    let duration_render = start_render.elapsed();

    // 3. Save
    pb.set_style(ProgressStyle::default_spinner().template("{spinner:.green} {msg}").unwrap());
    pb.set_message(format!("Saving {}.png...", layer.name));

    let start_save = Instant::now();
    let output_path = config.output_dir.join(format!("{}.png", layer.name));
    renderer.save(&output_path)?;
    let duration_save = start_save.elapsed();

    let total_layer = start_layer.elapsed();
    pb.finish_with_message(format!(
        "  Layer {}: done in {:.2?} (Read: {:.2?}, Render: {:.2?}, Save: {:.2?})",
        layer.name, total_layer, duration_read, duration_render, duration_save
    ));

    Ok(())
}
