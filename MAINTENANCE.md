# Simple Crosshair Overlay Maintenance

This document contains some common commands that I need to remember for package maintenance.

## Outdated Dependencies

1. Check for outdated dependencies: `cargo outdated --workspace`
2. Update dependencies: `cargo update`

## Dependency Vulnerability/License Check

`cargo deny --workspace check`

## Tests

`cargo test --workspace`

## Bloat Measurement

1. Temporarily comment out `strip = true` in [Cargo.toml](Cargo.toml)
2. Run `+nightly bloat -Z build-std=std --target x86_64-pc-windows-msvc --release -n 50 --crates`

## Benchmarks

See [crosshair-lib/benches](crosshair-lib/benches) for details.

## Profiling

1. add `debug = true` to `[profile.release]`
2. remove `strip = true` from `[profile.release]`
3. elevate to administrator privileges
4. `cargo flamegraph`

## Size-Optimized Build Using Nightly Rust

This is how I actually build the project for releases:
`cargo +nightly build -Z build-std=std --release`

This saves some binary filesize by allowing us to link-time-optimize against the standard lib. 
See [min-sized-rust](https://github.com/johnthagen/min-sized-rust) for a full explanation.
