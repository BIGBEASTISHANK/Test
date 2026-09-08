use rftmle::manifest::ManifestAction;
use std::path::Path;
use std::thread;

fn main() {
    // Initial manifest checking
    if Path::new(rftmle::MANIFEST_FILE).exists() {
        println!("Manifest exists");
    } else {
        match rftmle::manifest::manifest(ManifestAction::Initialize) {
            Ok(msg) => println!("{}", msg),
            Err(e) => println!("Error: {}", e),
        }
    }

    // File monitor initiate
    thread::spawn(|| {
        rftmle::fileMonitor::FileMonitor().unwrap();
    });

    // Preventing the exit of progra.
    loop {
       thread::park();
    }
}
