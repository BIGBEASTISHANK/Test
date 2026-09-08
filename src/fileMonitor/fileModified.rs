use crate::manifest;

pub fn FileModified(fileName: String) {
    println!("File Modified: {}", fileName);

    match manifest::manifest(manifest::ManifestAction::Update(fileName)) {
        Ok(msg) => {
            println!("{}", msg);
        }

        Err(e) => {
            eprintln!("Manifest Update Error: {}", e);
        }
    }
}
