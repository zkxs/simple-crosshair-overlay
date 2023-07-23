use std::io;

static ICON_NAME: &str = "target/app.ico";

fn main() -> io::Result<()> {
    // only generate Windows resource info on Windows.
    // also, don't run this on debug builds because some IDEs will furiously re-run build.rs constantly.
    // if cfg!(target_os = "windows") {
    if cfg!(all(target_os = "windows", not(debug_assertions))) {

        emit_ico_layers().unwrap();

        winres::WindowsResource::new()
            .set_icon(ICON_NAME)
            .set("ProductName", "Simple Crosshair Overlay")
            .set("FileDescription", "Simple Crosshair Overlay")
            .set("InternalName", "Simple Crosshair Overlay")
            .set("LegalCopyright", "Copyright © 2023 Michael Ripley")
            .set_language(0x0009) // english
            .compile()?;
    }

    Ok(())
}


#[cfg(target_os = "windows")]
fn emit_ico_layers() -> io::Result<()> {
    let mut icon_dir = ico::IconDir::new(ico::ResourceType::Icon);

    for size in [16, 24, 32, 48, 64, 256] {
        let image = ico::IconImage::from_rgba_data(size, size, generate_icon_rgba(size));
        icon_dir.add_entry(ico::IconDirEntry::encode(&image)?);
    }

    let file = std::fs::File::create(ICON_NAME)?;
    icon_dir.write(file)
}

// terribly copy-pasted from main
//TODO: don't duplicate this into main
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
