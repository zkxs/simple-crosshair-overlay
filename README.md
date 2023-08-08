# Simple Crosshair Overlay

A simple native crosshair overlay without unnecessary bloat. Free and open-source software.


![screenshot of the default, simple crosshair in action](screenshots/cross.png)


<details>
<summary>Click here to expand another screenshot demoing a custom PNG crosshair</summary>

![screenshot of a custom PNG crosshair](screenshots/custom.png)

</details>

## Features

- Works on any application that's not fullscreen exclusive. You **must** use windowed or borderless-windowed mode on your game. This was an intentional design choice, as rendering into a fullscreen-exclusive game is not anticheat-compatible.
- Performant: the overlay is only redrawn when you change the crosshair.
- Minimal UI: managed via a tray icon and hotkeys.
- Comes with a simple default crosshair that can be scaled to your preference
- Can use custom PNG images as crosshairs. Alpha is not only supported: it's mandatory. Because why would you want an opaque rectangle as your crosshair?
- Crosshair settings are saved when you exit the application.
- No sprawling installation. The only file this program uses is small configuration saved in `%appdata%\simple-crosshair-overlay`.

## Installation

1. Download simple-crosshair-overlay.exe from the [latest release](https://github.com/zkxs/simple-crosshair-overlay/releases/latest), and save it to a location of your choice
2. Run simple-crosshair-overlay.exe
3. Optionally, if you want a start menu shortcut you can make one yourself! Simply right-click simple-crosshair-overlay.exe and select "Pin to Start". This will automatically create a shortcut in `%appdata%\Microsoft\Windows\Start Menu\Programs`. 

Binaries are also provided for MacOS, although they are untested. If you're interested in helping test see [issue #3](https://github.com/zkxs/simple-crosshair-overlay/issues/3).

Linux is presently unsupported. See [issue #6](https://github.com/zkxs/simple-crosshair-overlay/issues/6).

## Usage

Use the tray icon to:

- toggle crosshair visibility (you can also use Ctrl+H)
- toggle **Adjust Mode** (you can also use Ctrl+J)
- load a PNG image as your crosshair
- reset crosshair to default settings
- safely exit the application and save your settings

In **Adjust Mode**, use the arrow keys to move the crosshair and PageUp/PageDown to increase/decrease the crosshair scale.

### Manual Config Editing

The config file is saved to `%appdata%\simple-crosshair-overlay\config\config.toml`. The following settings currently
cannot be edited in-application and can only be changed via manual config file editing.

**Color** of the default crosshair can be manually edited by  changing the `color` setting in `config.toml` to an ARGB
hexadecimal value. So for example `B2FF0000` for red with a bit of transparency, or `FF00FF00` for fully opaque green.
Note that this has no effect on custom PNG crosshairs.

**Hotkeys** can be manually edited by changing configs in the `key_bindings` section of `config.toml` using the Keycode
values defined in [keycode.rs](src/hotkey/keycode.rs).

## Installing from Source

1. [Install Rust](https://www.rust-lang.org/tools/install)
2. `cargo install simple-crosshair-overlay`

## Building from Source

1. [Install Rust](https://www.rust-lang.org/tools/install)
2. Clone the project
3. `cargo build --release`
   or alternatively for a slightly smaller binary, `cargo +nightly build -Z build-std=std --release`.
   See [min-sized-rust](https://github.com/johnthagen/min-sized-rust) for an explanation.

## License

Copyright 2023 [Michael Ripley](https://github.com/zkxs).

Simple Crosshair Overlay is provided under the [GPL-3.0 license](LICENSE).
