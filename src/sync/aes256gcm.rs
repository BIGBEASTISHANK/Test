use aes_gcm::{
    Aes256Gcm,
    KeyInit,
    Nonce,
    aead::Aead,
};
use rand::RngCore;

pub fn EncryptFile(
    data: &[u8]
) -> Result<(Vec<u8>, [u8; 12]), Box<dyn std::error::Error>> {

    // ==========================================
    // LOAD COMMON ENCRYPTION KEY
    // ==========================================

    let key =
        std::fs::read(
            crate::COMMON_ENCRYPTION_KEY_LOCATION
        )?;

    if key.len() != 32 {
        return Err(
            "Common encryption key must be 32 bytes"
                .into()
        );
    }

    // ==========================================
    // CONVERT TO [u8; 32]
    // ==========================================

    let key_array:
        [u8; 32] =
        key.as_slice()
            .try_into()
            .map_err(
                |_| "Invalid AES-256 key length"
            )?;

    // ==========================================
    // CREATE CIPHER
    // ==========================================

    let cipher =
        Aes256Gcm::new(
            (&key_array).into()
        );

    // ==========================================
    // GENERATE NONCE
    // ==========================================

    let mut nonce_bytes =
        [0u8; 12];

    rand::rng()
        .fill_bytes(
            &mut nonce_bytes
        );

    let nonce =
        Nonce::from_slice(
            &nonce_bytes
        );

    // ==========================================
    // ENCRYPT
    // ==========================================

    let encrypted =
        cipher
            .encrypt(
                nonce,
                data
            )
            .map_err(
                |_| "AES-256-GCM encryption failed"
            )?;

    Ok(
        (
            encrypted,
            nonce_bytes
        )
    )
}