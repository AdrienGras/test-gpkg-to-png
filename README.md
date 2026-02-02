# 🦀 gpkg-to-png 🖼️

[![Rust](https://img.shields.io/badge/rust-v1.70+-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Release](https://github.com/AdrienGras/test-gpkg-to-png/actions/workflows/release.yml/badge.svg)](https://github.com/AdrienGras/test-gpkg-to-png/releases)
[![Claude Code](https://img.shields.io/badge/Built%20with-Claude%20Code-blue.svg)](https://claude.ai/code)

> A blazing-fast Rust CLI tool to transform your GeoPackage and GeoJSON files into beautiful transparent PNG overlays. 🚀

---

## ✨ Features

- 📦 **Multiple Formats**: Supports GeoPackage (`.gpkg`) and GeoJSON (`.geojson`) files.
- 📚 **Multi-Layer Reading**: Automatically extracts polygons/multipolygons (GPKG) or geometries (GeoJSON).
- 🌍 **On-the-Fly Reprojection**: Automatic conversion to WGS84 (EPSG:4326) using `proj` for GPKG files.
- 🎨 **Flexible Styling**: Fully configurable fill (RGBA) and stroke (RGB) colors.
- 📐 **High Precision**: Customizable resolution in degrees per pixel or scale in meters per pixel.
- 🏎️ **Rust Performance**: Parallelized rendering for optimal execution speed.

## 🚀 Installation

### 📋 Prerequisites
- [Rust](https://www.rust-lang.org/tools/install) (2021 edition)
- Cargo

### 🏗️ Building from Source
```bash
git clone https://github.com/AdrienGras/test-gpkg-to-png.git
cd test-gpkg-to-png
cargo build --release
```
The executable will be available at `target/release/gpkg-to-png`.

> 💡 **Tip**: You can also download pre-built binaries for Linux from the [Releases](https://github.com/AdrienGras/test-gpkg-to-png/releases) section of this repository.

## 🛠️ Usage

```bash
gpkg-to-png <INPUT> [OPTIONS]
```

### ⚙️ Main Options

| Option           | Shortcut | Description                                                             | Default                   |
| :--------------- | :-------- | :---------------------------------------------------------------------- | :------------------------ |
| `<INPUT>`        |           | **Argument**: Path to `.gpkg` or `.geojson` file                        |                           |
| `--format`       | `-f`      | Input format: `gpkg` or `geojson`                                       | **Required**              |
| `--verbose`      | `-v`      | Verbose mode with timestamps and colored logs                           |                           |
| `--quiet`        | `-q`      | Quiet mode (only outputs file paths)                                    |                           |
| `--no-color`     |           | Disable ANSI colors (auto-detected for non-TTY)                         |                           |
| `--output-dir`   | `-o`      | Output directory                                                        | `.`                       |
| `--bbox`         | `-b`      | Bounding box: `minLon,minLat,maxLon,maxLat`                             | *Auto-detected if omitted*|
| `--resolution`   | `-r`      | Pixel size in degrees (mutually exclusive with `--scale`)               |                           |
| `--scale`        | `-s`      | Scale in meters per pixel (mutually exclusive with `--resolution`)      |                           |
| `--fill`         |           | Fill color RGBA hex (e.g., `FF000080`)                                  | `FF000080`                |
| `--stroke`       |           | Stroke color RGB hex (e.g., `FF0000`)                                   | `FF0000`                  |
| `--stroke-width` |           | Stroke width in pixels                                                  | `1`                       |
| `--layer`        | `-l`      | Specific layer name to render (GPKG only)                               | *All*                     |
| `--output-name`  |           | Output PNG filename (GeoJSON only)                                      | *Input filename*          |
| `--help`         | `-h`      | Display help                                                            |                           |
| `--version`      | `-V`      | Display version                                                         |                           |

> **Note**: You must specify either `--resolution` or `--scale`. If `bbox` is not provided, the tool will auto-detect it from the data extent.

### 💡 Examples

**Render a GeoPackage with custom colors:**
```bash
gpkg-to-png zones.gpkg \
  -f gpkg \
  --bbox "-4.5,48.0,-4.0,48.5" \
  --resolution 0.0001 \
  --fill "00FF0080" \
  --stroke "00FF00" \
  --stroke-width 2 \
  -o ./output/
```

**Render a GeoJSON with automatic resolution:**
```bash
gpkg-to-png data.geojson \
  -f geojson \
  --scale 10 \
  --output-name "my-overlay" \
  -o ./output/
```

**Render a specific layer in a GPKG:**
```bash
gpkg-to-png zones.gpkg \
  -f gpkg \
  --layer "parcels" \
  --resolution 0.0001 \
  -o ./output/
```

**Verbose mode with detailed timestamps:**
```bash
gpkg-to-png zones.gpkg \
  -f gpkg \
  -v \
  --resolution 0.0001 \
  -o ./output/
# Output: [0.00s] [INFO] Auto-detecting bounding box...
#         [0.02s] [DEBUG] Rendering geometry 1/100
#         ...
```

**Quiet mode (for scripts):**
```bash
gpkg-to-png zones.gpkg -f gpkg -q --resolution 0.0001 -o ./output/
# Output: ./output/zones.png
```

## 🏗️ Project Architecture

```text
src/
├── main.rs       // 🏗️ Entry point & format dispatch
├── cli.rs        // ⌨️ Argument parsing with clap
├── gpkg.rs       // 📂 GeoPackage reading & reprojection
├── geojson.rs    // 🌐 GeoJSON reading (WGS84)
├── render.rs     // 🎨 Rendering algorithms (Scanline/Bresenham)
├── render/
│   └── edge.rs   // 📊 Scanline edge table management
├── math.rs       // 📐 Coordinate transformations
└── error.rs      // 🚨 Robust error handling
```

## 🛠️ Dependencies

The project leverages the best tools in the Rust ecosystem:
- `sqlx` & `tokio` for asynchronous data access.
- `geo` & `proj` for geospatial manipulation.
- `geojson` for GeoJSON parsing.
- `image` for high-performance raster rendering.
- `rayon` for massive parallelism.
- `atty` for TTY detection (automatic colors).

## 🧪 Testing

```bash
cargo test                 # ✅ Unit tests (48 tests)
cargo test --test integration -- --ignored # 🔍 GPKG integration tests
cargo test --test geojson_integration -- --ignored # 🌐 GeoJSON integration tests
```

---

## 📜 License

MIT © [Adrien Gras](https://github.com/AdrienGras)

---

## 🧪 About this POC: The "Vibe Coding" Approach

This project is more than just a technical tool—it's a **proof of concept** exploring a new way of building software: **Vibe Coding**.

The goal was to test the productivity and relevance of an end-to-end AI-assisted development stack.

### 🛠️ Development Stack Used:
- **Orchestration & Execution**: [Claude Code](https://claude.ai/code) (the CLI agent that wrote these lines).
- **Intelligence & "Vibes"**: A dynamic mix via **OpenRouter**, primarily using **Claude 4.5 Sonnet** (Anthropic) and **Gemini 3 Flash** (Google).
- **Process**: No code was written by hand. Every feature, from choosing the scanline fill algorithm to managing parallelism with `rayon`, was proposed, discussed, and implemented by AI under user supervision.

### 📊 Experience Report:
- ⏱️ **Total Time**: About **5 hours**, including design, implementation, debugging, and documentation.
- 💰 **Cost**: About **€60** in API tokens (OpenRouter / Anthropic).
- ✅ **Result**: Robust, typed, performant, and fully documented Rust code.

*This project demonstrates that with the right AI tools and a clear vision, you can transform an idea into a viable tool in record time.* 🚀
