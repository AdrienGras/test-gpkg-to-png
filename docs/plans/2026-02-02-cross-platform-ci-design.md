# Cross-Platform CI/CD Build Design

**Date**: 2026-02-02  
**Author**: Claude  
**Status**: Ready for Implementation

## Overview

Extend the existing release workflow to build native binaries for all major platforms: Linux (x86_64, ARM64), macOS (Intel, Apple Silicon), and Windows (x64).

## Goals

- Build natively on each platform for maximum compatibility
- Use GitHub Actions matrix strategy for parallelization
- Optimize build times with caching
- Run only essential tests in CI (unit tests only)
- Produce stripped binaries for smaller release artifacts
- Support both Git tag triggers (existing behavior)

## Architecture

### Build Matrix

| Platform | Runner | Rust Target | Output Suffix |
|----------|--------|-------------|---------------|
| Linux x86_64 | `ubuntu-latest` | `x86_64-unknown-linux-gnu` | `linux-amd64` |
| Linux ARM64 | `ubuntu-latest` | `aarch64-unknown-linux-gnu` | `linux-arm64` |
| macOS Intel | `macos-13` | `x86_64-apple-darwin` | `macos-amd64` |
| macOS Apple Silicon | `macos-14` | `aarch64-apple-darwin` | `macos-arm64` |
| Windows x64 | `windows-latest` | `x86_64-pc-windows-msvc` | `windows-amd64.exe` |

### Workflow Structure

```
┌─────────────────────────────────────────────────────────────┐
│                    Trigger: Tag v*                          │
└─────────────────────────────────────────────────────────────┘
                            │
        ┌───────────────────┼───────────────────┐
        │                   │                   │
        ▼                   ▼                   ▼
┌──────────────┐   ┌──────────────┐   ┌──────────────┐
│ Linux x86_64 │   │ Linux ARM64  │   │ macOS Intel  │
│    Build     │   │    Build     │   │    Build     │
└──────────────┘   └──────────────┘   └──────────────┘
        │                   │                   │
        └───────────────────┼───────────────────┘
                            │
        ┌───────────────────┼───────────────────┐
        │                   │                   │
        ▼                   ▼                   ▼
┌──────────────┐   ┌──────────────┐   ┌──────────────┐
│ macOS ARM64  │   │ Windows x64  │   │   Collect    │
│    Build     │   │    Build     │   │   & Release  │
└──────────────┘   └──────────────┘   └──────────────┘
```

All five builds run in parallel. The "Collect & Release" job waits for all builds to succeed, then creates a single GitHub release with all binaries and a checksums file.

## Platform-Specific Configuration

### Linux (x86_64 and ARM64)

Both targets build on `ubuntu-latest` with cross-compilation for ARM64.

**Dependencies**:
```bash
sudo apt-get update
sudo apt-get install -y cmake clang libsqlite3-dev pkg-config
# For ARM64 cross-compilation:
sudo apt-get install -y gcc-aarch64-linux-gnu
```

**Cross-compilation setup**:
```bash
rustup target add aarch64-unknown-linux-gnu
export CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc
export AR_aarch64_unknown_linux_gnu=aarch64-linux-gnu-ar
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
```

**PROJ handling**: proj-sys builds PROJ from source during cargo build (bundled feature enabled by default).

### macOS (Intel and Apple Silicon)

**Dependencies**:
```bash
brew install cmake proj
```

**Runner selection**:
- `macos-13` for Intel (x86_64) builds
- `macos-14` for Apple Silicon (ARM64) builds (native ARM64 runner)

**PROJ handling**: Homebrew provides pre-built PROJ library, faster than building from source.

### Windows (x64)

**Dependencies** (via vcpkg):
```powershell
vcpkg install proj:x64-windows-static-md sqlite3:x64-windows-static-md
```

**Environment setup**:
```powershell
$env:VCPKG_ROOT = "C:\vcpkg"
$env:PATH = "$env:VCPKG_ROOT;$env:PATH"
```

**PROJ handling**: vcpkg provides pre-built PROJ library. Static linking with dynamic CRT (`static-md`) for standalone executable.

**Why vcpkg?**:
- Reliable, well-maintained package for PROJ
- Handles all transitive dependencies automatically
- Caching supported via `actions/cache`

## Testing Strategy

All platforms run only **unit tests** to keep CI fast:

```yaml
- name: Run unit tests
  run: cargo test --lib
```

**Why unit tests only?**:
- Integration tests require 6.1 MB test.gpkg file management in CI
- Unit tests cover core functionality: geometry parsing, rendering, math
- Integration tests remain available for local execution: `cargo test --test integration -- --ignored`
- Saves 2-3 minutes per build job

**Post-build validation**:
```yaml
- name: Verify binary
  run: |
    ./gpkg-to-png --version
    ./gpkg-to-png --help
```

## Artifact Management

### Naming Convention

```
gpkg-to-png-{VERSION}-{PLATFORM}-{ARCH}
```

Examples:
- `gpkg-to-png-v1.2.3-linux-amd64`
- `gpkg-to-png-v1.2.3-linux-arm64`
- `gpkg-to-png-v1.2.3-macos-amd64`
- `gpkg-to-png-v1.2.3-macos-arm64`
- `gpkg-to-png-v1.2.3-windows-amd64.exe`

### Stripping Binaries

All binaries are stripped to remove debug symbols:

**Linux/macOS**:
```bash
strip gpkg-to-png
```

**Windows**:
```powershell
# Already stripped by default in release profile
# Or use: strip gpkg-to-png.exe (if available)
```

**Size reduction**: ~30% smaller binaries (from ~15MB to ~10MB typical).

### Checksums

A `checksums.txt` file is generated with SHA256 hashes:

```
SHA256 (gpkg-to-png-v1.2.3-linux-amd64) = a1b2c3d4...
SHA256 (gpkg-to-png-v1.2.3-linux-arm64) = e5f6g7h8...
...
```

## Caching Strategy

### Rust Dependencies

Use `Swatinem/rust-cache@v2` for all platforms:
- Caches `target/` directory
- Caches Cargo registry index
- Keyed by OS and Cargo.lock hash

### Platform-Specific Caches

**Windows vcpkg**:
```yaml
- uses: actions/cache@v4
  with:
    path: C:\vcpkg\installed
    key: vcpkg-proj-${{ runner.os }}-${{ hashFiles('.github/workflows/release.yml') }}
```

**macOS Homebrew**:
Homebrew packages are cached by the runner image, no additional cache needed.

## Workflow Jobs

### 1. Build Matrix (5 parallel jobs)

Each job:
1. Checkout code
2. Install platform dependencies
3. Set up Rust toolchain (with target for cross-compilation)
4. Restore caches
5. Run unit tests (`cargo test --lib`)
6. Build release binary (`cargo build --release`)
7. Strip binary
8. Rename with platform suffix
9. Upload artifact

### 2. Release Job

Depends on all 5 builds completing successfully:

1. Download all artifacts
2. Generate checksums.txt
3. Create GitHub release with:
   - All 5 binaries
   - checksums.txt
   - Auto-generated release notes
   - Draft: false, Prerelease: false

## Error Handling

### Build Failures

If any platform build fails:
- Other parallel builds continue
- Release job does not run
- No partial release created
- Artifacts from successful builds available in workflow logs

### Dependency Installation Failures

**Linux**: apt-get failures fail the job immediately (required dependencies)
**macOS**: Homebrew failures fail the job immediately
**Windows**: vcpkg failures fail the job immediately

### Test Failures

Unit test failures on any platform fail that build job immediately, preventing a broken binary from being released.

## Security Considerations

### Token Permissions

Use minimal `contents: write` permission for release job only:
```yaml
permissions:
  contents: write
```

Build jobs need no special permissions.

### Binary Provenance

- All binaries built from tagged commits
- Build logs publicly visible in GitHub Actions
- Checksums provided for verification
- No external binary downloads (except vcpkg packages)

## Estimated Build Times

| Platform | Estimated Time |
|----------|----------------|
| Linux x86_64 | 4-5 minutes |
| Linux ARM64 | 7-8 minutes |
| macOS Intel | 5-6 minutes |
| macOS ARM64 | 4-5 minutes |
| Windows x64 | 10-12 minutes (includes vcpkg) |

**Total wall-clock time**: ~12 minutes (limited by Windows build)

## Future Enhancements

Optional improvements for later:

1. **Code signing**: Sign macOS and Windows binaries
2. **Homebrew formula**: Auto-update Homebrew tap on release
3. **Chocolatey package**: Windows package manager support
4. **Nightly builds**: Weekly builds from main branch
5. **MUSL builds**: Static Linux binaries (no glibc dependency)
6. **ARM64 Windows**: When GitHub provides ARM64 Windows runners

## Implementation Notes

### Critical Dependencies

- **PROJ**: Must be available on all platforms (via proj-sys build or system package)
- **SQLite**: Required by PROJ, bundled or system-provided
- **CMake**: Required to build PROJ from source

### Known Challenges

1. **Windows vcpkg**: First run downloads and builds dependencies (~8-10 min), subsequent runs use cache (~2-3 min)
2. **ARM64 cross-compile**: Slower than native builds due to emulation/toolchain overhead
3. **macOS runners**: Limited availability, may queue during busy periods

### Testing the Workflow

Before merging:
1. Create a test tag: `git tag v0.0.0-test && git push origin v0.0.0-test`
2. Verify all 5 builds complete successfully
3. Download and test binaries on each platform
4. Delete test tag and release after verification

## Success Criteria

- [ ] All 5 platform builds complete successfully on tag push
- [ ] Unit tests pass on all platforms
- [ ] Binaries are stripped and < 15MB each
- [ ] Release created with all binaries and checksums
- [ ] Binaries run successfully on target platforms (`--version`, `--help`)
- [ ] No regression in existing Linux release behavior
