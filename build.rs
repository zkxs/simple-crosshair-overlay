// This file is part of simple-crosshair-overlay and is licenced under the GNU GPL v3.0.
// See LICENSE file for full text.
// Copyright © 2023 Michael Ripley

use std::{env, fs, io};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

/// Tray icon dimension. [As per Microsoft](https://learn.microsoft.com/en-us/windows/win32/shell/notification-area?redirectedfrom=MSDN#add-a-notification-icon):
///
/// > An application should provide both a 16x16 pixel icon and a 32x32 icon
///
/// Yeah, the tray-icon bindings don't make passing multiple sizes easy so I'm simply going to **not do that**.
///
/// 32*32*4 = 4096, so this adds 4k to my binary filesize.
const TRAY_ICON_DIMENSION: u32 = 32;

/// The sexy Windows .ico with the multiple size defined below adds ~26k to the binary.
const APP_ICON_DIMENSIONS: [u32; 5] = [16, 24, 32, 48, 64];

static CONSTANTS_SOURCE_NAME: &str = "constants.rs";
static TRAY_ICON_NAME: &str = "trayicon.argb";
static APP_ICON_NAME: &str = "app.ico";
static APP_NAME: &str = "Simple Crosshair Overlay";

// Put in some indication that a build was in debug profile so there's a chance someone with the wrong build might one day notice
static APP_NAME_DEBUG: &str = if cfg!(debug_assertions) {
    "Simple Crosshair Overlay [DEBUG BUILD]"
} else {
    APP_NAME
};

fn main() -> io::Result<()> {
    let out_dir: PathBuf = env::var("OUT_DIR").expect("bad out dir?").into();

    // generate build constants
    {
        let constants_path = out_dir.join(CONSTANTS_SOURCE_NAME);
        create_constants(constants_path.as_path())?;
        println!("cargo:rustc-env=CONSTANTS_PATH={}", constants_path.to_str().unwrap());
    }

    // generate a tray icon
    {
        let tray_icon_path = out_dir.join(TRAY_ICON_NAME);
        generate_file_if_not_cached(tray_icon_path.as_path(), create_tray_icon_file)?;
        println!("cargo:rustc-env=TRAY_ICON_PATH={}", tray_icon_path.to_str().unwrap());
    }

    // only generate Windows resource info on Windows.
    if cfg!(target_os = "windows") {
        let icon_path = out_dir.join(APP_ICON_NAME);
        generate_file_if_not_cached(icon_path.as_path(), create_windows_app_icon_file)?;

        winres::WindowsResource::new()
            .set_icon(icon_path.to_str().expect("bad icon path?"))
            .set("ProductName", APP_NAME)
            .set("FileDescription", APP_NAME_DEBUG) // Windows presents this to users in a few places. Notably file properties and Task Manager.
            .set("InternalName", APP_NAME)
            .set("LegalCopyright", "Copyright © 2023 Michael Ripley")
            .set_language(0x0009) // english
            .compile()?;
    }

    Ok(())
}

/// helper to cache results because some IDEs will furiously re-run build.rs constantly.
fn generate_file_if_not_cached<F, R>(path: &Path, generator: F) -> io::Result<Option<R>>
    where F: FnOnce(&Path) -> io::Result<R> {
    // never cache for release builds. They're so infrequent it's not worth the risk of using a stale asset.
    if cfg!(not(debug_assertions)) || !path.is_file() {
        let result = generator(path)?;
        println!("generated {}", path.display());
        Ok(Some(result))
    } else {
        Ok(None)
    }
}

/// generate rust source to send constants into the actual build
fn create_constants(path: &Path) -> io::Result<()> {
    let file = fs::File::create(path)?;
    let mut writer = BufWriter::new(file);
    writer.write_fmt(format_args!("pub const TRAY_ICON_DIMENSION: u32 = {TRAY_ICON_DIMENSION};\n"))?;
    writer.write_fmt(format_args!("pub const APPLICATION_NAME: &str = {APP_NAME_DEBUG:?};\n"))?;
    writer.flush()
}

/// build a tray icon as raw RGBA bytes
fn create_tray_icon_file(path: &Path) -> io::Result<()> {
    let file = fs::File::create(path)?;
    let mut writer = BufWriter::new(file);
    let vec = generate_icon_rgba(TRAY_ICON_DIMENSION);
    writer.write_all(&vec)?;
    writer.flush()
}

/// build a .ico file for windows
#[cfg(target_os = "windows")]
fn create_windows_app_icon_file(path: &Path) -> io::Result<()> {
    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);

    for size in APP_ICON_DIMENSIONS {
        let image = ico::IconImage::from_rgba_data(size, size, generate_icon_rgba(size));
        icon_dir.add_entry(ico::IconDirEntry::encode(&image)?);
    }

    let file = fs::File::create(path)?;
    icon_dir.write(file)
}

// TODO: stop doing absurd buffer math to generate icons and just freaking bake an SVG
/// Generate a simple icon. Just a red circle with a little green/blue gradient stuff going on to spice it up.
fn generate_icon_rgba(size: u32) -> Vec<u8> {
    // some silly math to make a colored circle
    let icon_size_squared = size * size;
    let mut icon_rgba: Vec<u8> = Vec::with_capacity((icon_size_squared * 4) as usize);
    #[allow(clippy::uninit_vec)]
    unsafe {
        // there is no requirement I build my image in a zeroed buffer.
        icon_rgba.set_len(icon_rgba.capacity());
    }
    for x in 0..size {
        for y in 0..size {
            let x_term = ((x as i32) * 2 - (size as i32) + 1) / 2;
            let y_term = ((y as i32) * 2 - (size as i32) + 1) / 2;
            let distance_squared = x_term * x_term + y_term * y_term;
            let mask: u8 = if distance_squared < icon_size_squared as i32 / 4 {
                0xFF
            } else {
                0x00
            };
            let icon_offset: usize = (x as usize * size as usize + y as usize) * 4;
            icon_rgba[icon_offset] = mask; // set red
            icon_rgba[icon_offset + 1] = (x * 128 / size) as u8 & mask; // set green
            icon_rgba[icon_offset + 2] = (y * 128 / size) as u8 & mask; // set blue
            icon_rgba[icon_offset + 3] = mask; // set alpha
        }
    }
    icon_rgba
}
