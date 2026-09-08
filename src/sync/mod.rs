#![allow(nonstandard_style)]

use std::fs;
use std::path::Path;

use pnet::datalink::{self, Channel::Ethernet};
use pnet::packet::ethernet::{EtherType, MutableEthernetPacket};

use pnet::packet::Packet;
use pnet::util::MacAddr;

pub mod aes256gcm;

// ==========================================
// CUSTOM ETHERNET PROTOCOL
// ==========================================

const SYNC_ETHERTYPE: u16 = 0x88B5;

const MAGIC: &[u8; 4] = b"SYNC";

const VERSION: u8 = 1;

// ==========================================
// HEADER SIZE
//
// MAGIC       4
// VERSION     1
// NAME LEN    2
// MANIFEST    4
// FILE LEN    8
// NONCE       12
//
// TOTAL       31
// ==========================================

const HEADER_SIZE: usize = 31;

// ==================================================
// SYNC
// ==================================================

pub fn Sync(file_name: String) -> Result<(), Box<dyn std::error::Error>> {
    // ==========================================
    // FILE PATH
    // ==========================================

    let file_path = format!("{}{}", crate::STORAGE_LOCATION, file_name);

    println!("Syncing file: {}", file_path);

    // ==========================================
    // CHECK FILE
    // ==========================================

    if !Path::new(&file_path).exists() {
        return Err(format!("File does not exist: {}", file_path).into());
    }

    // ==========================================
    // READ FILE
    // ==========================================

    let file_data = fs::read(&file_path)?;

    println!("Original file size: {} bytes", file_data.len());

    // ==========================================
    // READ CLIENT MANIFEST
    // ==========================================

    let manifest = fs::read(crate::MANIFEST_FILE)?;

    println!("Manifest size: {} bytes", manifest.len());

    // ==========================================
    // ENCRYPT FILE
    // ==========================================

    let (encrypted_file, nonce) = aes256gcm::EncryptFile(&file_data)?;

    println!("Encrypted file size: {} bytes", encrypted_file.len());

    println!("Nonce: {:02x?}", nonce);

    // ==========================================
    // FILE NAME BYTES
    // ==========================================

    let file_name_bytes = file_name.as_bytes();

    // ==========================================
    // VALIDATE FILE NAME LENGTH
    // ==========================================

    if file_name_bytes.len() > u16::MAX as usize {
        return Err("File name is too long".into());
    }

    // ==========================================
    // VALIDATE MANIFEST LENGTH
    // ==========================================

    if manifest.len() > u32::MAX as usize {
        return Err("Manifest is too large".into());
    }

    // ==========================================
    // VALIDATE FILE LENGTH
    // ==========================================

    if encrypted_file.len() > u64::MAX as usize {
        return Err("Encrypted file is too large".into());
    }

    // ==========================================
    // BUILD PAYLOAD
    // ==========================================

    let mut payload = Vec::with_capacity(
        HEADER_SIZE + file_name_bytes.len() + encrypted_file.len() + manifest.len(),
    );

    // ==========================================
    // MAGIC
    // ==========================================

    payload.extend_from_slice(MAGIC);

    // ==========================================
    // VERSION
    // ==========================================

    payload.push(VERSION);

    // ==========================================
    // FILE NAME LENGTH
    // ==========================================

    payload.extend_from_slice(&(file_name_bytes.len() as u16).to_be_bytes());

    // ==========================================
    // MANIFEST LENGTH
    // ==========================================

    payload.extend_from_slice(&(manifest.len() as u32).to_be_bytes());

    // ==========================================
    // ENCRYPTED FILE LENGTH
    // ==========================================

    payload.extend_from_slice(&(encrypted_file.len() as u64).to_be_bytes());

    // ==========================================
    // NONCE
    // ==========================================

    payload.extend_from_slice(&nonce);

    // ==========================================
    // FILE NAME
    // ==========================================

    payload.extend_from_slice(file_name_bytes);

    // ==========================================
    // ENCRYPTED FILE
    // ==========================================

    payload.extend_from_slice(&encrypted_file);

    // ==========================================
    // MANIFEST
    // ==========================================

    payload.extend_from_slice(&manifest);

    println!("SYNC payload size: {} bytes", payload.len());

    // ==========================================
    // FIND NETWORK INTERFACE
    // ==========================================

    let interfaces = datalink::interfaces();

    let server_mac = parse_mac(crate::SERVER_MAC_ADDRESS)?;

    let interface = interfaces
        .into_iter()
        .find(|interface| {
            interface.is_up()
                && !interface.is_loopback()
                && interface.mac.map_or(false, |mac| mac != MacAddr::zero())
        })
        .ok_or("No suitable network interface found")?;

    println!(
        "Sync using interface: {} ({:?})",
        interface.name, interface.mac
    );

    // ==========================================
    // GET CLIENT MAC
    // ==========================================

    let source_mac = interface
        .mac
        .ok_or("Network interface has no MAC address")?;

    // ==========================================
    // ETHERNET FRAME SIZE
    //
    // Ethernet header = 14 bytes
    // ==========================================

    let ethernet_frame_size = 14 + payload.len();

    let mut ethernet_buffer = vec![0u8; ethernet_frame_size];

    // ==========================================
    // CREATE ETHERNET PACKET
    // ==========================================

    let mut ethernet_packet = MutableEthernetPacket::new(&mut ethernet_buffer)
        .ok_or("Failed to create Ethernet packet")?;

    // ==========================================
    // DESTINATION MAC
    // ==========================================

    ethernet_packet.set_destination(server_mac);

    // ==========================================
    // SOURCE MAC
    // ==========================================

    ethernet_packet.set_source(source_mac);

    // ==========================================
    // CUSTOM ETHERTYPE
    //
    // IMPORTANT:
    // EtherType::new(...)
    //
    // NOT:
    // EtherTypes::new(...)
    // ==========================================

    ethernet_packet.set_ethertype(EtherType::new(SYNC_ETHERTYPE));

    // ==========================================
    // PAYLOAD
    // ==========================================

    ethernet_packet.set_payload(&payload);

    // ==========================================
    // OPEN LAYER-2 CHANNEL
    // ==========================================

    let (mut tx, _) = match datalink::channel(&interface, Default::default())? {
        Ethernet(tx, rx) => (tx, rx),

        _ => {
            return Err("Unsupported datalink channel".into());
        }
    };

    // ==========================================
    // SEND ETHERNET FRAME
    // ==========================================

    tx.send_to(ethernet_packet.packet(), None)
        .ok_or("Failed to send Ethernet packet")??;

    println!("Sync sent successfully: {}", file_name);

    Ok(())
}

// ==================================================
// MAC ADDRESS PARSER
// ==================================================

fn parse_mac(mac: &str) -> Result<MacAddr, Box<dyn std::error::Error>> {
    let parts: Vec<&str> = mac.split(':').collect();

    if parts.len() != 6 {
        return Err(format!("Invalid MAC address: {}", mac).into());
    }

    let mut bytes = [0u8; 6];

    for i in 0..6 {
        bytes[i] = u8::from_str_radix(parts[i], 16)
            .map_err(|_| format!("Invalid MAC address: {}", mac))?;
    }

    Ok(MacAddr::new(
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5],
    ))
}
