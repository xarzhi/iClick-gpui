fn main() -> Result<(), Box<dyn std::error::Error>> {
    winres::WindowsResource::new()
        .set_icon("icon.ico")
        .compile()?;
    Ok(())
}
