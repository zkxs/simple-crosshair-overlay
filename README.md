# Simple Crosshair Overlay

A simple native crosshair overlay without unnecessary bloat. Free and open-source software.


![screenshot of the default, simple crosshair in action](screenshots/cross.png)


<details>
<summary>Click here to expand another screenshot demoing a custom PNG crosshair</summary>

![screenshot of a custom PNG crosshair](screenshots/custom.png)

</details>

## Features

- Works on any application that's *not* fullscreen exclusive. You **must** use windowed or borderless-windowed mode on your game. This was an intentional design choice, as rendering into a fullscreen-exclusive game is *not* anticheat-compatible.
- Performant: the overlay is only redrawn when you change the crosshair.
- Minimal UI: managed via a tray icon and hotkeys.
- Comes with a simple default crosshair that can be scaled to your preference
- Can use custom PNG images as crosshairs. Alpha is not only supported: it's mandatory. Because why would you want an opaque rectangle as your crosshair?
- Crosshair settings are saved when you exit the application.
- No sprawling installation. simple-crosshair-overlay.exe itself and a single config file placed in `%appdata%\simple-crosshair-overlay` are all the files this program needs.

## Installation

1. Download simple-crosshair-overlay.exe from the [latest release](https://github.com/zkxs/simple-crosshair-overlay/releases/latest), and save it to a location of your choice
2. Run simple-crosshair-overlay.exe
3. Optionally, if you want a start menu shortcut you can make one yourself! Simply right-click simple-crosshair-overlay.exe and select "Pin to Start". This will automatically create a shortcut in `%appdata%\Microsoft\Windows\Start Menu\Programs`. 

## Usage

Use the tray icon to:

- toggle crosshair visibility (you can also use Ctrl+H)
- toggle **Adjust Mode** (you can also use Ctrl+J)
- load a PNG image as your crosshair
- reset crosshair to default settings
- safely exit the application and save your settings

In **Adjust Mode**, use the arrow keys to move the crosshair and PageUp/PageDown to increase/decrease the crosshair scale.

Color cannot currently be changed in-application. However, it can be manually altered by editing the `color` setting in
`%appdata%\simple-crosshair-overlay\config\config.toml` to an ARGB hexadecimal value.
So for example `B2FF0000` for red with a bit of transparency, or `FF00FF00` for fully opaque green.

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
