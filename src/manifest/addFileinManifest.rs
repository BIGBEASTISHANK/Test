use std::collections::HashMap;
use std::fs;

use crate::MANIFEST_FILE;

pub fn AddFileinManifest(
    fileName: String,
) -> Result<String, std::io::Error> {

    // Read existing manifest
    let mut manifest: crate::ManifestStruct = if fs::exists(MANIFEST_FILE)? {
        let data = fs::read_to_string(MANIFEST_FILE)?;

        serde_json::from_str(&data).map_err(|error| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                error,
            )
        })?
    } else {
        crate::ManifestStruct(HashMap::new())
    };

    let lastEditId = 0;

    // Add file to manifest
    manifest.0.insert(
        fileName,
        crate::ManifestFile {
            lastEditId
        },
    );

    // Convert manifest to JSON
    let json = serde_json::to_string_pretty(&manifest).map_err(|error| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            error,
        )
    })?;

    // Save manifest
    fs::write(MANIFEST_FILE, json)?;

    Ok("File added successfully".to_string())
}