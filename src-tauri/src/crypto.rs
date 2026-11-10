//! 本机 VNC 密码的 AES-256-GCM 封装。
//!
//! 密钥在第一次启动时随机生成，放在配置目录的 `secret.key`（Unix 上权限 0600）。
//! 这不是操作系统钥匙串：拷走整个配置目录就能解密。好处是不依赖
//! 桌面会话里的 keyring 服务，测试和无头环境也能跑。密文格式是
//! `base64(12 字节 nonce || ciphertext)`。

use std::fs;
use std::path::Path;

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use rand::RngCore;

const KEY_LEN: usize = 32;
const NONCE_LEN: usize = 12;

pub struct Vault {
    key: [u8; KEY_LEN],
}

impl Vault {
    pub fn load_or_create(path: &Path) -> Result<Self, String> {
        if path.exists() {
            let bytes = fs::read(path).map_err(|err| format!("读取密钥失败: {err}"))?;
            if bytes.len() != KEY_LEN {
                return Err(format!(
                    "密钥文件长度异常（{} 字节），为避免把已保存的密码全部作废，没有自动覆盖它",
                    bytes.len()
                ));
            }
            let mut key = [0u8; KEY_LEN];
            key.copy_from_slice(&bytes);
            return Ok(Self { key });
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|err| format!("创建密钥目录失败: {err}"))?;
        }
        let mut key = [0u8; KEY_LEN];
        rand::rngs::OsRng.fill_bytes(&mut key);
        fs::write(path, key).map_err(|err| format!("写入密钥失败: {err}"))?;
        restrict_private(path);
        Ok(Self { key })
    }

    pub fn encrypt(&self, plaintext: &str) -> Result<String, String> {
        let cipher = Aes256Gcm::new_from_slice(&self.key).map_err(|err| err.to_string())?;
        let mut nonce_bytes = [0u8; NONCE_LEN];
        rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|err| format!("加密失败: {err}"))?;
        let mut packed = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        packed.extend_from_slice(&nonce_bytes);
        packed.extend_from_slice(&ciphertext);
        Ok(STANDARD.encode(packed))
    }

    pub fn decrypt(&self, encoded: &str) -> Result<String, String> {
        let packed = STANDARD
            .decode(encoded.trim())
            .map_err(|err| format!("密码密文不是合法的 base64: {err}"))?;
        if packed.len() <= NONCE_LEN {
            return Err("密码密文太短".into());
        }
        let (nonce_bytes, ciphertext) = packed.split_at(NONCE_LEN);
        let cipher = Aes256Gcm::new_from_slice(&self.key).map_err(|err| err.to_string())?;
        let nonce = Nonce::from_slice(nonce_bytes);
        let plain = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| "无法解密已保存的 VNC 密码（密钥是否被换过？）".to_string())?;
        String::from_utf8(plain).map_err(|_| "解密结果不是 UTF-8 文本".to_string())
    }
}

fn restrict_private(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_tamper() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("secret.key");
        let vault = Vault::load_or_create(&path).unwrap();
        let encoded = vault.encrypt("s3cret-密码").unwrap();
        assert!(!encoded.contains("s3cret"));
        assert_eq!(vault.decrypt(&encoded).unwrap(), "s3cret-密码");

        let again = Vault::load_or_create(&path).unwrap();
        assert_eq!(again.decrypt(&encoded).unwrap(), "s3cret-密码");

        let mut raw = STANDARD.decode(&encoded).unwrap();
        let last = raw.len() - 1;
        raw[last] ^= 0x01;
        let tampered = STANDARD.encode(raw);
        assert!(vault.decrypt(&tampered).is_err());
    }

    #[test]
    fn refuses_short_key_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("secret.key");
        fs::write(&path, [1u8, 2, 3]).unwrap();
        assert!(Vault::load_or_create(&path).is_err());
    }
}
