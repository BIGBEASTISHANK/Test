#![allow(nonstandard_style)]

use aes_gcm::{Aes256Gcm, KeyInit, Nonce, aead::Aead};
use pgp::{Message, SignedSecretKey, Deserializable};
use std::io::Cursor;

// ==================================================
// DECRYPT RECEIVED FILE
//
// Flow:
//   1. Read .server.enc.key  → PGP-decrypt → DEK (raw bytes)
//   2. Read .server.enc      → AES-256-GCM decrypt with DEK
//                              (first 12 bytes = nonce, rest = ciphertext)
//                              → intermediate (common-key-encrypted data)
//   3. Intermediate          → AES-256-GCM decrypt with common key
//                              (first 12 bytes = nonce, rest = ciphertext)
//                              → plaintext
//   4. Write plaintext to storage (filename without .server.enc suffix)
//   5. Remove temporary .server.enc and .server.enc.key files
// ==================================================

pub fn DecryptReceivedFile(
    file_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {

    let storage = crate::STORAGE_LOCATION;

    let enc_file_path = format!("{}{}", storage, file_name);
    let dek_file_path = format!("{}{}.key", storage, file_name);

    // ==========================================
    // STEP 1: READ & PGP-DECRYPT THE DEK
    // ==========================================

    println!("Reading encrypted DEK from: {}", dek_file_path);

    let encrypted_dek = std::fs::read(&dek_file_path)?;

    println!("Encrypted DEK size: {} bytes", encrypted_dek.len());

    let dek = DecryptWithPGP(&encrypted_dek)?;

    println!("Decrypted DEK size: {} bytes", dek.len());

    if dek.len() != 32 {
        return Err(
            format!(
                "DEK must be 32 bytes for AES-256-GCM, got {} bytes",
                dek.len()
            )
            .into(),
        );
    }

    // ==========================================
    // STEP 2: READ & DECRYPT .server.enc WITH DEK
    //
    // Layout: [12-byte nonce][AES-256-GCM ciphertext]
    // ==========================================

    println!("Reading DEK-encrypted file from: {}", enc_file_path);

    let dek_encrypted_data = std::fs::read(&enc_file_path)?;

    println!("DEK-encrypted file size: {} bytes", dek_encrypted_data.len());

    if dek_encrypted_data.len() < 12 {
        return Err("DEK-encrypted file is too small to contain a nonce".into());
    }

    let dek_nonce = Nonce::from_slice(&dek_encrypted_data[..12]);

    let dek_cipher = Aes256Gcm::new_from_slice(&dek)
        .map_err(|e| format!("Failed to build DEK cipher: {}", e))?;

    let intermediate = dek_cipher
        .decrypt(dek_nonce, &dek_encrypted_data[12..])
        .map_err(|_| "DEK AES-256-GCM decryption failed")?;

    println!("Intermediate (common-key-encrypted) size: {} bytes", intermediate.len());

    // ==========================================
    // STEP 3: DECRYPT INTERMEDIATE WITH COMMON KEY
    //
    // Layout: [12-byte nonce][AES-256-GCM ciphertext]
    // ==========================================

    if intermediate.len() < 12 {
        return Err(
            "Intermediate data is too small to contain a nonce".into()
        );
    }

    let common_key = std::fs::read(crate::COMMON_ENCRYPTION_KEY_LOCATION)?;

    if common_key.len() != 32 {
        return Err(
            format!(
                "Common key must be 32 bytes, got {} bytes",
                common_key.len()
            )
            .into(),
        );
    }

    let common_nonce = Nonce::from_slice(&intermediate[..12]);

    let common_cipher = Aes256Gcm::new_from_slice(&common_key)
        .map_err(|e| format!("Failed to build common cipher: {}", e))?;

    let plaintext = common_cipher
        .decrypt(common_nonce, &intermediate[12..])
        .map_err(|_| "Common key AES-256-GCM decryption failed")?;

    println!("Decrypted plaintext size: {} bytes", plaintext.len());

    // ==========================================
    // STEP 4: DETERMINE OUTPUT FILE NAME
    //
    // Strip ".server.enc" suffix to get original name
    // ==========================================

    let output_name = file_name
        .strip_suffix(".server.enc")
        .unwrap_or(file_name);

    let output_path = format!("{}{}", storage, output_name);

    // ==========================================
    // STEP 5: MARK OUTPUT FILE AS RECEIVING
    // So the file monitor does not sync it back
    // ==========================================

    crate::fileMonitor::MarkReceivingFile(output_name.to_string());

    // ==========================================
    // STEP 6: WRITE PLAINTEXT
    // ==========================================

    std::fs::write(&output_path, &plaintext)?;

    println!("Plaintext saved to: {}", output_path);

    // ==========================================
    // STEP 7: CLEAN UP TEMPORARY ENCRYPTED FILES
    // ==========================================

    std::fs::remove_file(&enc_file_path)?;
    std::fs::remove_file(&dek_file_path)?;

    println!("Removed temporary encrypted files");

    Ok(())
}

// ==================================================
// PGP DECRYPTION HELPER
//
// Supports both ASCII-armored and binary PGP keys.
// Assumes the private key has no passphrase.
// ==================================================

fn DecryptWithPGP(
    encrypted_data: &[u8],
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {

    let key_bytes = std::fs::read(crate::PGP_PRIVATE_KEY_LOCATION)?;

    // ==========================================
    // LOAD SECRET KEY
    // ==========================================

    let secret_key: SignedSecretKey = if key_bytes.starts_with(b"-----") {

        // ASCII-armored key
        let key_str = std::str::from_utf8(&key_bytes)?;
        let (key, _) = SignedSecretKey::from_string(key_str)?;
        key

    } else {

        // Binary key
        let (key, _) = SignedSecretKey::from_bytes(
            Cursor::new(key_bytes)
        )?;
        key

    };

    // ==========================================
    // PARSE PGP MESSAGE
    // ==========================================

    let (msg, _) = Message::from_bytes(
        Cursor::new(encrypted_data)
    )?;

    // ==========================================
    // DECRYPT MESSAGE (no passphrase)
    // ==========================================

    let (decrypted_msg, _) = msg.decrypt(
        || String::new(),
        &[&secret_key],
    )?;

    // ==========================================
    // EXTRACT CONTENT BYTES
    // ==========================================

    let content = decrypted_msg
        .get_content()?
        .ok_or("PGP message contained no decryptable content")?;

    Ok(content)
}
