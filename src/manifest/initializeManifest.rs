use std::fs::File;
use std::io::Write;

pub fn InitializeManifest() -> Result<String, std::io::Error> {
    let mut file = File::create(crate::MANIFEST_FILE)?;

    file.write_all(b"{}")?;

    Ok("Manifest initialized successfully".to_string())
}