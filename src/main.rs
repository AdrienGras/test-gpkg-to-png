mod cli;
mod error;
mod gpkg;
mod math;
mod render;

use clap::Parser;

use cli::Args;
use error::{GpkgError, Result};
use gpkg::GpkgReader;
use render::{RenderConfig, Renderer};

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

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

    println!("Processing {} layer(s)...", layers_to_process.len());

    // Process each layer
    for layer in &layers_to_process {
        println!("  Layer: {}", layer.name);

        // Read and reproject geometries
        let geometries = reader.read_geometries_wgs84(layer).await?;
        println!("    {} geometries", geometries.len());

        if geometries.is_empty() {
            println!("    Skipping: no geometries");
            continue;
        }

        // Create renderer
        let render_config = RenderConfig {
            bbox: config.bbox,
            resolution: config.resolution,
            fill: config.fill,
            stroke: config.stroke,
            stroke_width: config.stroke_width,
        };

        let mut renderer = Renderer::new(render_config)?;
        let (width, height) = renderer.dimensions();
        println!("    Image: {}x{} pixels", width, height);

        // Render all geometries
        for geom in &geometries {
            renderer.render_multipolygon(geom);
        }

        // Save output
        let output_path = config.output_dir.join(format!("{}.png", layer.name));
        renderer.save(&output_path)?;
        println!("    Saved: {}", output_path.display());
    }

    println!("Done!");
    Ok(())
}
