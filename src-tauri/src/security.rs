use std::{fs, path::Path};

use aes_gcm::{
    aead::{Aead, KeyInit, Payload},
    Aes256Gcm, Nonce,
};
use argon2::Argon2;
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, Zeroizing};

use crate::{
    domain::{validate_pin, validate_recovery_password},
    error::{AppError, AppResult},
};

const KEYRING_SERVICE: &str = "sn.kerfinance.multiservices";
const KEYRING_USER: &str = "manager-device-secret";
const LOCAL_AAD: &[u8] = b"ker-finance/local-key/v1";
const RECOVERY_AAD: &[u8] = b"ker-finance/recovery-key/v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyEnvelope {
    pub version: u8,
    pub local_salt: String,
    pub local_nonce: String,
    pub local_ciphertext: String,
    pub recovery_salt: String,
    pub recovery_nonce: String,
    pub recovery_ciphertext: String,
}

fn random_bytes<const N: usize>() -> [u8; N] {
    let mut bytes = [0u8; N];
    rand::rng().fill_bytes(&mut bytes);
    bytes
}

fn derive_key(secret: &[u8], salt: &[u8]) -> AppResult<Zeroizing<[u8; 32]>> {
    let mut key = Zeroizing::new([0u8; 32]);
    Argon2::default()
        .hash_password_into(secret, salt, key.as_mut())
        .map_err(|e| AppError::Security(format!("Dérivation de clé impossible: {e}")))?;
    Ok(key)
}

fn encrypt_key(
    database_key: &[u8],
    wrapping_key: &[u8],
    nonce: &[u8; 12],
    aad: &[u8],
) -> AppResult<Vec<u8>> {
    let cipher =
        Aes256Gcm::new_from_slice(wrapping_key).map_err(|e| AppError::Security(e.to_string()))?;
    cipher
        .encrypt(
            Nonce::from_slice(nonce),
            Payload {
                msg: database_key,
                aad,
            },
        )
        .map_err(|_| AppError::Security("Chiffrement de la clé impossible.".into()))
}

fn decrypt_key(
    encrypted: &[u8],
    wrapping_key: &[u8],
    nonce: &[u8],
    aad: &[u8],
) -> AppResult<Zeroizing<Vec<u8>>> {
    if nonce.len() != 12 {
        return Err(AppError::Security("Enveloppe de clé invalide.".into()));
    }
    let cipher =
        Aes256Gcm::new_from_slice(wrapping_key).map_err(|e| AppError::Security(e.to_string()))?;
    cipher
        .decrypt(
            Nonce::from_slice(nonce),
            Payload {
                msg: encrypted,
                aad,
            },
        )
        .map(Zeroizing::new)
        .map_err(|_| AppError::Security("Déchiffrement de la clé impossible.".into()))
}

fn keyring_entry() -> AppResult<keyring::Entry> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .map_err(|e| AppError::Security(format!("Coffre sécurisé Windows indisponible: {e}")))
}

fn create_device_secret() -> AppResult<Zeroizing<Vec<u8>>> {
    let secret = random_bytes::<32>().to_vec();
    keyring_entry()?.set_secret(&secret).map_err(|e| {
        AppError::Security(format!(
            "Impossible de protéger la clé sur cet appareil: {e}"
        ))
    })?;
    Ok(Zeroizing::new(secret))
}

fn get_device_secret() -> AppResult<Zeroizing<Vec<u8>>> {
    keyring_entry()?
        .get_secret()
        .map(Zeroizing::new)
        .map_err(|e| {
            AppError::Security(format!(
                "Secret de cet appareil introuvable. Utilisez une sauvegarde et le mot de passe de récupération: {e}"
            ))
        })
}

fn local_secret(pin: &str, device_secret: &[u8]) -> Zeroizing<Vec<u8>> {
    let mut value = Zeroizing::new(Vec::with_capacity(pin.len() + device_secret.len() + 1));
    value.extend_from_slice(device_secret);
    value.push(0);
    value.extend_from_slice(pin.as_bytes());
    value
}

pub fn create_key_envelope(
    pin: &str,
    recovery_password: &str,
) -> AppResult<(KeyEnvelope, Zeroizing<Vec<u8>>)> {
    validate_pin(pin)?;
    validate_recovery_password(recovery_password)?;
    let device_secret = create_device_secret()?;
    let database_key = Zeroizing::new(random_bytes::<32>().to_vec());
    let envelope =
        create_envelope_with_device(&database_key, pin, recovery_password, &device_secret)?;
    Ok((envelope, database_key))
}

fn create_envelope_with_device(
    database_key: &[u8],
    pin: &str,
    recovery_password: &str,
    device_secret: &[u8],
) -> AppResult<KeyEnvelope> {
    let local_salt = random_bytes::<16>();
    let local_nonce = random_bytes::<12>();
    let recovery_salt = random_bytes::<16>();
    let recovery_nonce = random_bytes::<12>();

    let mut pin_secret = local_secret(pin, device_secret);
    let local_key = derive_key(&pin_secret, &local_salt)?;
    pin_secret.zeroize();
    let recovery_key = derive_key(recovery_password.as_bytes(), &recovery_salt)?;

    Ok(KeyEnvelope {
        version: 1,
        local_salt: B64.encode(local_salt),
        local_nonce: B64.encode(local_nonce),
        local_ciphertext: B64.encode(encrypt_key(
            database_key,
            local_key.as_ref(),
            &local_nonce,
            LOCAL_AAD,
        )?),
        recovery_salt: B64.encode(recovery_salt),
        recovery_nonce: B64.encode(recovery_nonce),
        recovery_ciphertext: B64.encode(encrypt_key(
            database_key,
            recovery_key.as_ref(),
            &recovery_nonce,
            RECOVERY_AAD,
        )?),
    })
}

fn decode(value: &str) -> AppResult<Vec<u8>> {
    B64.decode(value)
        .map_err(|_| AppError::Security("Enveloppe de clé invalide.".into()))
}

pub fn unlock_with_pin(envelope: &KeyEnvelope, pin: &str) -> AppResult<Zeroizing<Vec<u8>>> {
    validate_pin(pin)?;
    let device_secret = get_device_secret()?;
    let mut pin_secret = local_secret(pin, &device_secret);
    let salt = decode(&envelope.local_salt)?;
    let nonce = decode(&envelope.local_nonce)?;
    let ciphertext = decode(&envelope.local_ciphertext)?;
    let local_key = derive_key(&pin_secret, &salt)?;
    pin_secret.zeroize();
    decrypt_key(&ciphertext, local_key.as_ref(), &nonce, LOCAL_AAD)
        .map_err(|_| AppError::InvalidPin)
}

pub fn unlock_with_recovery(
    envelope: &KeyEnvelope,
    recovery_password: &str,
) -> AppResult<Zeroizing<Vec<u8>>> {
    validate_recovery_password(recovery_password)?;
    let salt = decode(&envelope.recovery_salt)?;
    let nonce = decode(&envelope.recovery_nonce)?;
    let ciphertext = decode(&envelope.recovery_ciphertext)?;
    let recovery_key = derive_key(recovery_password.as_bytes(), &salt)?;
    decrypt_key(&ciphertext, recovery_key.as_ref(), &nonce, RECOVERY_AAD)
        .map_err(|_| AppError::InvalidRecoveryPassword)
}

pub fn rewrap_recovered_key(
    database_key: &[u8],
    pin: &str,
    recovery_password: &str,
) -> AppResult<KeyEnvelope> {
    validate_pin(pin)?;
    validate_recovery_password(recovery_password)?;
    let device_secret = match get_device_secret() {
        Ok(secret) => secret,
        Err(_) => create_device_secret()?,
    };
    create_envelope_with_device(database_key, pin, recovery_password, &device_secret)
}

pub fn read_envelope(path: &Path) -> AppResult<KeyEnvelope> {
    let bytes = fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

pub fn write_envelope(path: &Path, envelope: &KeyEnvelope) -> AppResult<()> {
    let parent = path
        .parent()
        .ok_or_else(|| AppError::Internal("Chemin de sécurité invalide.".into()))?;
    fs::create_dir_all(parent)?;
    let temp = parent.join("security.tmp");
    fs::write(&temp, serde_json::to_vec_pretty(envelope)?)?;
    fs::rename(temp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_roundtrip_and_wrong_password() {
        let db_key = [42u8; 32];
        let device = [7u8; 32];
        let envelope = create_envelope_with_device(
            &db_key,
            "123456",
            "une phrase de récupération solide",
            &device,
        )
        .unwrap();
        let recovered =
            unlock_recovery_without_keyring(&envelope, "une phrase de récupération solide")
                .unwrap();
        assert_eq!(recovered.as_slice(), db_key);
        assert!(
            unlock_recovery_without_keyring(&envelope, "mot de passe totalement faux").is_err()
        );
    }

    fn unlock_recovery_without_keyring(
        envelope: &KeyEnvelope,
        password: &str,
    ) -> AppResult<Zeroizing<Vec<u8>>> {
        let salt = decode(&envelope.recovery_salt)?;
        let nonce = decode(&envelope.recovery_nonce)?;
        let ciphertext = decode(&envelope.recovery_ciphertext)?;
        let key = derive_key(password.as_bytes(), &salt)?;
        decrypt_key(&ciphertext, key.as_ref(), &nonce, RECOVERY_AAD)
    }
}
