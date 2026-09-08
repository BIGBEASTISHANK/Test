use std::{collections::HashMap, fs, time::SystemTime};

pub fn LoadExistingFiles(files: &mut HashMap<String, SystemTime>) {
    let directory = match fs::read_dir(crate::STORAGE_LOCATION) {
        Ok(directory) => directory,

        Err(error) => {
            eprintln!("Could not read storage directory: {}", error);

            return;
        }
    };

    for entry in directory.flatten() {
        let path = entry.path();

        // Ignore directories
        if !path.is_file() {
            continue;
        }

        // Ignore hidden / temporary files
        if let Some(file_name) = path.file_name() {
            if file_name.to_string_lossy().starts_with('.') {
                continue;
            }
        }

        let file_name = path
            .to_string_lossy()
            .strip_prefix(crate::STORAGE_LOCATION)
            .unwrap()
            .to_string();

        let modified_time = match fs::metadata(&path).and_then(|metadata| metadata.modified()) {
            Ok(time) => time,

            Err(_) => continue,
        };

        files.insert(file_name, modified_time);
    }
}
