#![allow(nonstandard_style)]

use pnet::datalink::{self, Channel::Ethernet};
use pnet::packet::Packet;
use pnet::packet::ethernet::EthernetPacket;
use pnet::util::MacAddr;

use std::fs;
use std::path::Path;

// ==========================================
// SERVER → CLIENT FILE TRANSFER
// ==========================================

const FILE_TRANSFER_ETHERTYPE: u16 = 0x88B6;

const MAGIC: &[u8; 4] = b"FILE";

const VERSION: u8 = 1;

// ==========================================
// HEADER
//
// MAGIC       4
// VERSION     1
// NAME LEN    2
// DEK LEN     4
// FILE LEN    8
//
// TOTAL       19
// ==========================================

const HEADER_SIZE: usize = 19;

// ==========================================
// RECEIVE FILES
// ==========================================

pub fn ReceiveFiles() -> Result<(), Box<dyn std::error::Error>> {
    println!("==========================================");
    println!("Waiting for server file transfer...");
    println!("==========================================");

    // ==========================================
    // FIND CLIENT INTERFACE
    // ==========================================

    let interfaces = datalink::interfaces();

    let interface = interfaces
        .into_iter()
        .find(|interface| {
            interface.is_up()
                && !interface.is_loopback()
                && interface.mac.map_or(false, |mac| mac != MacAddr::zero())
        })
        .ok_or("No suitable network interface found")?;

    let client_mac = interface.mac.ok_or("Client interface has no MAC address")?;

    println!("Client listening on interface: {}", interface.name);

    println!("Client MAC: {}", client_mac);

    // ==========================================
    // OPEN LAYER-2 CHANNEL
    // ==========================================

    let (_, mut rx) = match datalink::channel(&interface, Default::default())? {
        Ethernet(tx, rx) => (tx, rx),

        _ => {
            return Err("Unsupported datalink channel".into());
        }
    };

    // ==========================================
    // RECEIVE LOOP
    // ==========================================

    loop {
        let packet = rx
            .next()
            .map_err(|e| format!("Failed to receive Ethernet packet: {}", e))?;

        // ==========================================
        // PARSE ETHERNET FRAME
        // ==========================================

        let ethernet_packet = match EthernetPacket::new(packet) {
            Some(packet) => packet,

            None => {
                println!("Received invalid Ethernet packet");

                continue;
            }
        };

        // ==========================================
        // CHECK DESTINATION MAC
        // ==========================================

        let destination = ethernet_packet.get_destination();

        if destination != client_mac && destination != MacAddr::broadcast() {
            continue;
        }

        // ==========================================
        // CHECK ETHERTYPE
        // ==========================================

        let ethertype = ethernet_packet.get_ethertype();

        if ethertype.0 != FILE_TRANSFER_ETHERTYPE {
            continue;
        }

        println!();
        println!("==========================================");
        println!("FILE TRANSFER PACKET RECEIVED");
        println!("==========================================");

        println!("Source MAC: {}", ethernet_packet.get_source());

        println!("Destination MAC: {}", destination);

        println!("EtherType: 0x{:04x}", ethertype.0);

        // ==========================================
        // GET PAYLOAD
        // ==========================================

        let payload = ethernet_packet.payload();

        // ==========================================
        // PROCESS FILE
        // ==========================================

        match ProcessFileTransfer(payload) {
            Ok(_) => {
                println!("File transfer processed successfully");

                println!("==========================================");
            }

            Err(error) => {
                eprintln!("Failed to process file transfer: {}", error);
            }
        }
    }

    Ok(())
}

// ==========================================
// PROCESS FILE TRANSFER
// ==========================================

fn ProcessFileTransfer(payload: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    // ==========================================
    // CHECK HEADER SIZE
    // ==========================================

    if payload.len() < HEADER_SIZE {
        return Err(format!("Payload too small: {} bytes", payload.len()).into());
    }

    let mut offset = 0;

    // ==========================================
    // MAGIC
    // ==========================================

    let magic = &payload[offset..offset + 4];

    offset += 4;

    if magic != MAGIC {
        return Err("Invalid FILE packet magic".into());
    }

    println!("Magic: FILE");

    // ==========================================
    // VERSION
    // ==========================================

    let version = payload[offset];

    offset += 1;

    if version != VERSION {
        return Err(format!("Unsupported FILE packet version: {}", version).into());
    }

    println!("Version: {}", version);

    // ==========================================
    // FILE NAME LENGTH
    // ==========================================

    let name_len = u16::from_be_bytes([payload[offset], payload[offset + 1]]) as usize;

    offset += 2;

    println!("File name length: {}", name_len);

    // ==========================================
    // DEK LENGTH
    // ==========================================

    let dek_len = u32::from_be_bytes([
        payload[offset],
        payload[offset + 1],
        payload[offset + 2],
        payload[offset + 3],
    ]) as usize;

    offset += 4;

    println!("Encrypted DEK length: {}", dek_len);

    // ==========================================
    // FILE LENGTH
    // ==========================================

    let file_len = u64::from_be_bytes([
        payload[offset],
        payload[offset + 1],
        payload[offset + 2],
        payload[offset + 3],
        payload[offset + 4],
        payload[offset + 5],
        payload[offset + 6],
        payload[offset + 7],
    ]) as usize;

    offset += 8;

    println!("Encrypted file length: {}", file_len);

    // ==========================================
    // VALIDATE COMPLETE PACKET
    // ==========================================

    let expected_size = HEADER_SIZE + name_len + dek_len + file_len;

    if payload.len() < expected_size {
        return Err(format!(
            "Incomplete FILE packet: expected {} bytes, received {} bytes",
            expected_size,
            payload.len()
        )
        .into());
    }

    // ==========================================
    // FILE NAME
    // ==========================================

    let file_name_bytes = &payload[offset..offset + name_len];

    offset += name_len;

    let file_name = std::str::from_utf8(file_name_bytes)
        .map_err(|_| "Invalid UTF-8 file name")?
        .to_string();

    println!("File name: {}", file_name);

    // ==========================================
    // VALIDATE FILE NAME
    // ==========================================

    if file_name.is_empty()
        || file_name.contains("..")
        || file_name.starts_with('/')
        || file_name.starts_with('\\')
    {
        return Err("Invalid received file name".into());
    }

    // ==========================================
    // ENCRYPTED DEK
    // ==========================================

    let encrypted_dek = &payload[offset..offset + dek_len];

    offset += dek_len;

    println!("Encrypted DEK received: {} bytes", encrypted_dek.len());

    // ==========================================
    // ENCRYPTED FILE
    // ==========================================

    let encrypted_file = &payload[offset..offset + file_len];

    println!("Encrypted file received: {} bytes", encrypted_file.len());

    // ==========================================
    // STORAGE LOCATION
    // ==========================================

    let storage_path = Path::new(crate::STORAGE_LOCATION);

    if !storage_path.exists() {
        fs::create_dir_all(storage_path)?;
    }

    // ==========================================
    // MARK ENCRYPTED FILES AS RECEIVING
    // So the file monitor does not process them
    // ==========================================

    let dek_file_name = format!("{}.key", file_name);

    crate::fileMonitor::MarkReceivingFile(file_name.clone());
    crate::fileMonitor::MarkReceivingFile(dek_file_name.clone());

    // ==========================================
    // SAVE ENCRYPTED FILE
    // ==========================================

    let file_path = storage_path.join(&file_name);

    fs::write(&file_path, encrypted_file)?;

    println!("Encrypted file stored: {}", file_path.display());

    // ==========================================
    // SAVE ENCRYPTED DEK
    // ==========================================

    let dek_path = storage_path.join(&dek_file_name);

    fs::write(&dek_path, encrypted_dek)?;

    println!("Encrypted DEK stored: {}", dek_path.display());

    // ==========================================
    // DECRYPT RECEIVED FILE
    // ==========================================

    println!("Starting decryption of received file: {}", file_name);

    if let Err(error) = crate::sync::decryptFile::DecryptReceivedFile(&file_name) {
        eprintln!("Decryption failed: {}", error);
    }

    println!("==========================================");
    println!("FILE TRANSFER COMPLETE");
    println!("==========================================");

    Ok(())
}
