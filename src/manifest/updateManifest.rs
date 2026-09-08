use std::fs;

use super::addFileinManifest::AddFileinManifest;
use crate::MANIFEST_FILE;

pub fn UpdateManifest(
    fileName: String,
) -> Result<String, std::io::Error> {
    // If manifest does not exist, create it using AddFileinManifest
    if !fs::exists(MANIFEST_FILE)? {
        return AddFileinManifest(fileName);
    }

    // Read existing manifest
    let data = fs::read_to_string(MANIFEST_FILE)?;

    let mut manifest: crate::ManifestStruct = serde_json::from_str(&data)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;

    // Check if file exists
    match manifest.0.get_mut(&fileName) {
        Some(manifestFile) => {
            // Increment lastEditId
            manifestFile.lastEditId += 1;
        }

        None => {
            // File doesn't exist, so add it with ID 0
            return AddFileinManifest(fileName);
        }
    }

    // Convert manifest to JSON
    let json = serde_json::to_string_pretty(&manifest)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;

    // Save updated manifest
    fs::write(MANIFEST_FILE, json)?;

    Ok("Manifest updated successfully".to_string())
}
