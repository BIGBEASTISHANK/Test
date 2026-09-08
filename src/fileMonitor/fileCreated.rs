use crate::manifest;

pub fn FileCreated(fileName: String) {
    println!("File Created: {}", fileName);

    match manifest::manifest(manifest::ManifestAction::AddFile(fileName)) {
        Ok(msg) => {
            println!("{}", msg);
        }

        Err(e) => {
            eprintln!("Manifest Add Error: {}", e);
        }
    }
}
