//! Passphrase-protected authenticated encryption for personal workspace packs.
//!
//! Uses PBKDF2-HMAC-SHA256 (600,000 iterations, 16-byte random salt) and
//! AES-256-GCM (12-byte random nonce, fixed domain AAD) via `ring`.

use std::num::NonZeroU32;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ring::{
    aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM, NONCE_LEN},
    pbkdf2,
    rand::{SecureRandom, SystemRandom},
};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

const KDF_NAME: &str = "pbkdf2-hmac-sha256";
const CIPHER_NAME: &str = "aes-256-gcm";
const KDF_ITERATIONS: u32 = 600_000;
const SALT_LEN: usize = 16;
const KEY_LEN: usize = 32;
const SECRETS_AAD: &[u8] = b"fyagent-workspace-pack-secrets/v1";
const MIN_PASSPHRASE_CHARS: usize = 6;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EncryptedSecretsEnvelope {
    pub kdf: String,
    pub iterations: u32,
    pub cipher: String,
    pub salt_b64: String,
    pub nonce_b64: String,
    pub ciphertext_b64: String,
}

fn derive_key(
    passphrase: &str,
    salt: &[u8],
    iterations: u32,
) -> Result<Zeroizing<[u8; KEY_LEN]>, String> {
    let iter = NonZeroU32::new(iterations).ok_or_else(|| "密钥派生迭代次数无效".to_string())?;
    let mut key = Zeroizing::new([0u8; KEY_LEN]);
    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA256,
        iter,
        salt,
        passphrase.as_bytes(),
        key.as_mut(),
    );
    Ok(key)
}

pub fn encrypt_secrets_json(
    plaintext: &[u8],
    passphrase: &str,
) -> Result<EncryptedSecretsEnvelope, String> {
    if passphrase.chars().count() < MIN_PASSPHRASE_CHARS {
        return Err(format!("加密口令至少需要 {MIN_PASSPHRASE_CHARS} 个字符"));
    }
    let rng = SystemRandom::new();
    let mut salt = [0u8; SALT_LEN];
    rng.fill(&mut salt)
        .map_err(|_| "无法生成安全随机盐值".to_string())?;
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rng.fill(&mut nonce_bytes)
        .map_err(|_| "无法生成安全随机数".to_string())?;

    let derived = derive_key(passphrase, &salt, KDF_ITERATIONS)?;
    let unbound = UnboundKey::new(&AES_256_GCM, derived.as_ref())
        .map_err(|_| "初始化加密算法失败".to_string())?;
    let key = LessSafeKey::new(unbound);
    let nonce = Nonce::assume_unique_for_key(nonce_bytes);

    let mut buffer = plaintext.to_vec();
    key.seal_in_place_append_tag(nonce, Aad::from(SECRETS_AAD), &mut buffer)
        .map_err(|_| "加密 API Key 数据失败".to_string())?;

    Ok(EncryptedSecretsEnvelope {
        kdf: KDF_NAME.to_string(),
        iterations: KDF_ITERATIONS,
        cipher: CIPHER_NAME.to_string(),
        salt_b64: BASE64.encode(salt),
        nonce_b64: BASE64.encode(nonce_bytes),
        ciphertext_b64: BASE64.encode(&buffer),
    })
}

pub fn decrypt_secrets_json(
    envelope: &EncryptedSecretsEnvelope,
    passphrase: &str,
) -> Result<Zeroizing<Vec<u8>>, String> {
    if envelope.kdf != KDF_NAME || envelope.cipher != CIPHER_NAME {
        return Err("不支持的迁移包加密算法版本".to_string());
    }
    if envelope.iterations < 100_000 || envelope.iterations > 2_000_000 {
        return Err("迁移包加密参数异常，拒绝解密".to_string());
    }
    if passphrase.is_empty() {
        return Err("请输入迁移包解密口令".to_string());
    }

    let salt = BASE64
        .decode(&envelope.salt_b64)
        .map_err(|_| "迁移包密钥盐值损坏".to_string())?;
    if salt.len() != SALT_LEN {
        return Err("迁移包密钥盐值长度无效".to_string());
    }
    let nonce_vec = BASE64
        .decode(&envelope.nonce_b64)
        .map_err(|_| "迁移包加密随机数损坏".to_string())?;
    let nonce_bytes: [u8; NONCE_LEN] = nonce_vec
        .try_into()
        .map_err(|_| "迁移包加密随机数长度无效".to_string())?;
    let mut ciphertext = BASE64
        .decode(&envelope.ciphertext_b64)
        .map_err(|_| "迁移包密文数据损坏".to_string())?;

    let derived = derive_key(passphrase, &salt, envelope.iterations)?;
    let unbound = UnboundKey::new(&AES_256_GCM, derived.as_ref())
        .map_err(|_| "初始化解密算法失败".to_string())?;
    let key = LessSafeKey::new(unbound);
    let nonce = Nonce::assume_unique_for_key(nonce_bytes);

    let plaintext = key
        .open_in_place(nonce, Aad::from(SECRETS_AAD), &mut ciphertext)
        .map_err(|_| "解密口令错误或密钥密文已被篡改".to_string())?;

    Ok(Zeroizing::new(plaintext.to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_encrypts_and_rejects_wrong_passphrase_or_tampering() {
        let payload = br#"{"models":[{"target":"codex","apiKey":"sk-secret-123"}]}"#;
        let envelope = encrypt_secrets_json(payload, "correct-horse-99").expect("encrypt");
        assert!(!envelope.ciphertext_b64.contains("sk-secret-123"));

        let decrypted = decrypt_secrets_json(&envelope, "correct-horse-99").expect("decrypt");
        assert_eq!(decrypted.as_slice(), payload);

        let wrong_err = decrypt_secrets_json(&envelope, "wrong-passphrase").unwrap_err();
        assert!(wrong_err.contains("口令错误"));

        let mut tampered = envelope.clone();
        let mut bytes = BASE64.decode(&tampered.ciphertext_b64).unwrap();
        bytes[0] ^= 0xff;
        tampered.ciphertext_b64 = BASE64.encode(bytes);
        assert!(decrypt_secrets_json(&tampered, "correct-horse-99").is_err());

        assert!(encrypt_secrets_json(payload, "123").is_err());
    }
}
