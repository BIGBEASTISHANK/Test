use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::SystemTime;

pub mod fileCreated;
pub mod fileModified;
pub mod loadExistingFiles;

pub fn FileMonitor() -> notify::Result<()> {
    let (tx, rx) = channel();

    let mut watcher = RecommendedWatcher::new(
        move |result: notify::Result<Event>| {
            tx.send(result).unwrap();
        },
        Config::default(),
    )?;

    watcher.watch(
        Path::new(crate::STORAGE_LOCATION),
        RecursiveMode::NonRecursive,
    )?;

    println!("File Monitor started: {}", crate::STORAGE_LOCATION);

    // Store files and their last modification time.
    let mut files: HashMap<String, SystemTime> = HashMap::new();

    // Load files that already exist when monitor starts.
    loadExistingFiles::LoadExistingFiles(&mut files);

    loop {
        match rx.recv() {
            Ok(Ok(event)) => {
                for path in event.paths {
                    // ==========================================
                    // IGNORE DIRECTORIES
                    // ==========================================

                    if path.is_dir() {
                        continue;
                    }

                    // ==========================================
                    // IGNORE HIDDEN / TEMPORARY FILES
                    // ==========================================

                    if let Some(file_name) = path.file_name() {
                        if file_name.to_string_lossy().starts_with('.') {
                            continue;
                        }
                    }

                    // ==========================================
                    // GET FILE NAME
                    // ==========================================

                    let full_path = path.to_string_lossy().to_string();

                    let file_name = full_path
                        .strip_prefix(crate::STORAGE_LOCATION)
                        .unwrap()
                        .to_string();

                    if file_name.is_empty() {
                        continue;
                    }

                    // ==========================================
                    // CHECK FILE
                    // ==========================================

                    let metadata = match fs::metadata(&path) {
                        Ok(metadata) => metadata,

                        Err(_) => {
                            if files.remove(&file_name).is_some() {
                                println!("File Deleted: {}", file_name);
                            }

                            continue;
                        }
                    };

                    let modified_time = match metadata.modified() {
                        Ok(time) => time,

                        Err(_) => continue,
                    };

                    // ==========================================
                    // FILE CREATED
                    // ==========================================

                    if !files.contains_key(&file_name) {
                        files.insert(file_name.clone(), modified_time);

                        fileCreated::FileCreated(file_name.clone());

                        continue;
                    }

                    // ==========================================
                    // FILE MODIFIED
                    // ==========================================

                    let old_modified_time = files.get(&file_name).unwrap();

                    if *old_modified_time != modified_time {
                        files.insert(file_name.clone(), modified_time);

                        fileModified::FileModified(file_name.clone());

                        std::thread::sleep(std::time::Duration::from_millis(100));

                        println!("SYNC");

                        if let Err(error) = crate::sync::Sync(file_name) {
                            eprintln!("Sync failed: {}", error);
                        }
                    }
                }
            }

            Ok(Err(error)) => {
                eprintln!("File Monitor Error: {:?}", error);
            }

            Err(error) => {
                eprintln!("Channel Error: {:?}", error);

                break;
            }
        }
    }

    Ok(())
}
