pub mod addFileinManifest;
pub mod initializeManifest;
pub mod updateManifest;

#[derive(PartialEq)]
pub enum ManifestAction {
    Initialize,

    Update(String),

    AddFile(String),
}

pub fn manifest(action: ManifestAction) -> Result<String, std::io::Error> {
    match action {
        ManifestAction::Initialize => {
            println!("Initializing Manifest");

            initializeManifest::InitializeManifest()
        }

        ManifestAction::AddFile(fileName) => {
            println!("Adding File to Manifest");

            addFileinManifest::AddFileinManifest(fileName)
        }

        ManifestAction::Update(fileName) => {
            println!("Updating Manifest");

            updateManifest::UpdateManifest(fileName)
        }
    }
}
