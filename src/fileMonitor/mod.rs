#![allow(nonstandard_style)]

use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};

use std::collections::{HashMap, HashSet};

use std::fs;

use std::path::Path;

use std::sync::mpsc::channel;

use std::time::SystemTime;

pub mod fileCreated;
pub mod fileModified;
pub mod loadExistingFiles;

// ==================================================
// FILES CURRENTLY BEING RECEIVED FROM SERVER
// ==================================================

use std::sync::{Mutex, OnceLock};

static RECEIVING_FILES: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

fn get_receiving_files() -> &'static Mutex<HashSet<String>> {
    RECEIVING_FILES.get_or_init(|| Mutex::new(HashSet::new()))
}

// ==================================================
// MARK FILE AS RECEIVING
// ==================================================

pub fn MarkReceivingFile(file_name: String) {
    let mut files = get_receiving_files()
        .lock()
        .unwrap();

    files.insert(file_name);
}

// ==================================================
// CHECK IF FILE IS RECEIVING
// ==================================================

pub fn IsReceivingFile(file_name: &str) -> bool {
    let files = get_receiving_files()
        .lock()
        .unwrap();

    files.contains(file_name)
}

// ==================================================
// REMOVE RECEIVING FILE
// ==================================================

pub fn RemoveReceivingFile(file_name: &str) {
    let mut files = get_receiving_files()
        .lock()
        .unwrap();

    files.remove(file_name);
}

// ==================================================
// FILE MONITOR
// ==================================================

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

    println!(
        "File Monitor started: {}",
        crate::STORAGE_LOCATION
    );

    // ==================================================
    // STORE FILES AND MODIFICATION TIMES
    // ==================================================

    let mut files: HashMap<String, SystemTime> = HashMap::new();

    // ==================================================
    // LOAD EXISTING FILES
    // ==================================================

    loadExistingFiles::LoadExistingFiles(&mut files);

    // ==================================================
    // EVENT LOOP
    // ==================================================

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
                    // IGNORE HIDDEN FILES
                    // ==========================================

                    if let Some(file_name) = path.file_name() {
                        if file_name
                            .to_string_lossy()
                            .starts_with('.')
                        {
                            continue;
                        }
                    }

                    // ==========================================
                    // GET FILE NAME
                    // ==========================================

                    let full_path =
                        path.to_string_lossy().to_string();

                    let file_name = full_path
                        .strip_prefix(crate::STORAGE_LOCATION)
                        .unwrap()
                        .to_string();

                    if file_name.is_empty() {
                        continue;
                    }

                    // ==========================================
                    // CHECK IF THIS FILE IS BEING RECEIVED
                    // ==========================================

                    if IsReceivingFile(&file_name) {

                        println!(
                            "Ignoring received server file: {}",
                            file_name
                        );

                        // Update its modification time so that
                        // future events are handled correctly.
                        if let Ok(metadata) = fs::metadata(&path) {
                            if let Ok(modified_time) =
                                metadata.modified()
                            {
                                files.insert(
                                    file_name.clone(),
                                    modified_time,
                                );
                            }
                        }

                        RemoveReceivingFile(&file_name);

                        continue;
                    }

                    // ==========================================
                    // CHECK FILE
                    // ==========================================

                    let metadata = match fs::metadata(&path) {

                        Ok(metadata) => metadata,

                        Err(_) => {

                            if files.remove(&file_name).is_some() {

                                println!(
                                    "File Deleted: {}",
                                    file_name
                                );
                            }

                            continue;
                        }
                    };

                    // ==========================================
                    // GET MODIFICATION TIME
                    // ==========================================

                    let modified_time =
                        match metadata.modified() {

                            Ok(time) => time,

                            Err(_) => continue,
                        };

                    // ==========================================
                    // FILE CREATED
                    // ==========================================

                    if !files.contains_key(&file_name) {

                        files.insert(
                            file_name.clone(),
                            modified_time,
                        );

                        fileCreated::FileCreated(
                            file_name.clone()
                        );

                        continue;
                    }

                    // ==========================================
                    // FILE MODIFIED
                    // ==========================================

                    let old_modified_time =
                        files.get(&file_name).unwrap();

                    if *old_modified_time != modified_time {

                        files.insert(
                            file_name.clone(),
                            modified_time,
                        );

                        fileModified::FileModified(
                            file_name.clone()
                        );

                        std::thread::sleep(
                            std::time::Duration::from_millis(100)
                        );

                        println!("SYNC");

                        if let Err(error) =
                            crate::sync::Sync(file_name)
                        {
                            eprintln!(
                                "Sync failed: {}",
                                error
                            );
                        }
                    }
                }
            }

            Ok(Err(error)) => {

                eprintln!(
                    "File Monitor Error: {:?}",
                    error
                );
            }

            Err(error) => {

                eprintln!(
                    "Channel Error: {:?}",
                    error
                );

                break;
            }
        }
    }

    Ok(())
}