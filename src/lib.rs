#![allow(nonstandard_style)]

use std::collections::HashMap;

pub mod manifest;
pub mod fileMonitor;
pub mod sync;

pub static STORAGE_LOCATION: &str = "/home/ishank/Documents/Hackathon/ClientAStorage/";
pub static MANIFEST_FILE: &str = "/home/ishank/Documents/Hackathon/manifest.json";
pub static COMMON_ENCRYPTION_KEY_LOCATION: &str = "/home/ishank/Documents/Hackathon/CK.bin";
pub static PGP_PRIVATE_KEY_LOCATION: &str = "/home/ishank/Documents/Hackathon/client_private.pgp";
pub static SERVER_MAC_ADDRESS: &str = "34:5A:60:94:2E:02";

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ManifestStruct(
    HashMap<String, ManifestFile>
);

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ManifestFile {
    lastEditId: u64,
}