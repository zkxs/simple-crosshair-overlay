# Simple Crosshair Overlay Maintenance

This document contains some common commands that I need to remember for package maintenance.

## Outdated Dependencies

1. Check for outdated dependencies: `cargo outdated`
2. Update dependencies: `cargo update`

## Dependency Vulnerability/License Check

`cargo deny check`

## Check & Lint

To check all supported targets for compilation/lint errors, run:

`cargo clippy --target x86_64-pc-windows-msvc --target x86_64-apple-darwin --target aarch64-apple-darwin --all-features --all-targets`

## Tests

`cargo test`

## Bloat Measurement

1. Install [cargo-bloat](https://github.com/RazrFalcon/cargo-bloat)
2. Run `cargo +nightly bloat -Z build-std=std --target x86_64-pc-windows-msvc --profile=release-optimized-debug -n 50 --crates`

## Benchmarks

Run [benchmarks](benches) with
`cargo bench --features "benchmark"`

## Profiling

1. Install [flamegraph](https://github.com/flamegraph-rs/flamegraph)
2. elevate to administrator privileges
3. `cargo flamegraph --profile=release-optimized-debug`

## Unused Dependencies

1. Install [cargo-udeps](https://github.com/est31/cargo-udeps)
2. Be able to compile for the target you want to test
3. This MUST be run under the nightly toolchain
4. `cargo +nightly udeps`

## Size-Optimized Build Using Nightly Rust

This is how I actually build the project for releases:
`cargo +nightly build -Z build-std=std --release`

This saves some binary filesize by allowing us to link-time-optimize against the standard lib. 
See [min-sized-rust](https://github.com/johnthagen/min-sized-rust) for a full explanation.
