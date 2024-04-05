# Simple Crosshair Overlay Maintenance

This document contains some common commands that I need to remember for package maintenance.

## Outdated Dependencies

1. Check for outdated dependencies: `cargo outdated`
2. Update dependencies: `cargo update`

## Dependency Vulnerability/License Check

`cargo deny check`

## Tests

`cargo test`

## Bloat Measurement

1. Temporarily comment out `strip = true` in [Cargo.toml](Cargo.toml)
2. Run `cargo +nightly bloat -Z build-std=std --target x86_64-pc-windows-msvc --profile=release-optimized-debug -n 50 --crates`

## Benchmarks

Run [benchmarks](benches) with
`cargo bench --features "benchmark"`

## Profiling

1. elevate to administrator privileges
2. `cargo flamegraph --profile=release-optimized-debug`

## Size-Optimized Build Using Nightly Rust

This is how I actually build the project for releases:
`cargo +nightly build -Z build-std=std --release`

This saves some binary filesize by allowing us to link-time-optimize against the standard lib. 
See [min-sized-rust](https://github.com/johnthagen/min-sized-rust) for a full explanation.
