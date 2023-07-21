use std::io;

fn main() -> io::Result<()> {
    if cfg!(target_os = "windows") {
        winres::WindowsResource::new()
            //.set_icon("test.ico") // TODO: icon path relative to project root
            .set("ProductName", "Simple Crosshair Overlay")
            .set("FileDescription", "Simple Crosshair Overlay")
            .set("InternalName", "Simple Crosshair Overlay")
            .set("LegalCopyright", "Copyright © 2023 Michael Ripley")
            .set_language(0x0009) // english
            .compile()?;
    }

    Ok(())
}
