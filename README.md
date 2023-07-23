# Simple Crosshair Overlay

A simple native crosshair overlay without unnecessary bloat. Free and open-source software.


![screenshot of the default, simple crosshair in action](screenshots/cross.png)


<details>
<summary>Click here to expand another screenshot demoing a custom PNG crosshair</summary>

![screenshot of a custom PNG crosshair](screenshots/custom.png)

</details>

## Features

- Performant: the overlay is only redrawn when you change the crosshair
- Saves crosshair settings when you exit the application
- Managed via a tray icon and hotkeys
- Comes with a simple default crosshair that can be scaled to your preference
- Can use custom PNG images as crosshairs. Alpha is not only supported: it's mandatory. Because why would you want an opaque rectangle as your crosshair?
- Works on any application that's *not* fullscreen exclusive. You **must** use windowed or borderless-windowed mode on your game. This was an intentional design choice, as rendering into a fullscreen-exclusive game is *not* anticheat-compatible.
- Supports both even and odd sized crosshairs. This is noteworthy, as some games center your gun on a single pixel, while others center it on the intersection *between* four pixels.

## Installation

1. Download the [latest release](https://github.com/zkxs/simple-crosshair-overlay/releases/latest) to a location of your choice.
2. Run simple-crosshair-overlay.exe

## Usage

Use the tray icon to:

- toggle crosshair visibility (you can also use Ctrl+H)
- toggle **Adjust Mode** (you can also use Ctrl+J)
- load a PNG image as your crosshair
- reset crosshair to default settings
- safely exit the application and save your settings

In **Adjust Mode**, the arrow keys to move the crosshair and PageUp/PageDown to increase/decrease the crosshair scale.

Color cannot currently be changed in-application. However, it can be manually altered by editing the `color` setting in
`%appdata%\simple-crosshair-overlay\config\config.toml` to an ARGB hexadecimal value.

## To-Do

Maybe one day I'll get around to these features:

- Support changing color of built-in crosshair _without_ manual config editing
- Verify if it works on MacOS/Linux
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
