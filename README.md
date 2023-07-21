# Simple Crosshair Overlay

A simple native crosshair overlay without unnecessary bloat. Free and open-source software.

## Features

- Performant: the overlay is only redrawn if you adjust the crosshair scale
- Saves crosshair settings when you exit the application
- Managed via a tray icon and hotkeys
- Works on any application that's *not* fullscreen exclusive. You **must** use windowed or borderless-windowed mode on your game. This was an intentional design choice, as rendering into a fullscreen-exclusive game is *not* anticheat-compatible.
- Supports both even and odd sized crosshairs. This is noteworthy, as some games center your gun on a single pixel, while others center it on the intersection *between* four pixels.
- Changing crosshair scale preserves the center position. Note that due to the even/odd case explained above this means it will appear to wiggle 0.5 pixels as you scale it.

## Installation

1. Download the [latest release](https://github.com/zkxs/simple-crosshair-overlay/releases/latest) to a location of your choice.
2. Run simple-crosshair-overlay.exe

## Usage

Use the tray icon to:

- toggle crosshair visibility (you can also use Ctrl+H)
- toggle **Adjust Mode** (you can also use Ctrl+J)
- reset crosshair to default settings
- safely exit the application and save your settings

In **Adjust Mode**, the arrow keys to move the crosshair and PageUp/PageDown to increase/decrease the crosshair scale.

Color cannot currently be changed in-application. However, it can be manually altered by editing the `color` setting in
`%appdata%\simple-crosshair-overlay\config\config.toml` to an ARGB hexadecimal value.

## To-Do

Maybe one day I'll get around to these features:

- Support loading custom PNG images as crosshairs
- Support changing color of built-in crosshair
- Confirm it works on MacOS/Linux
- Custom app icon in both exe and tray
- Customizable hotkeys

<!-- TODO: publish crate
## Installing from Source

1. [Install Rust](https://www.rust-lang.org/tools/install)
2. `cargo install simple-crosshair-overlay`
-->

## Building from Source

1. [Install Rust](https://www.rust-lang.org/tools/install)
2. Clone the project
3. `cargo build --release`

## License

Copyright 2022-2023 [Michael Ripley](https://github.com/zkxs).

Simple Crosshair Overlay is provided under the [GPL-3.0 license](LICENSE).
