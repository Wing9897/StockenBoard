//! Bot Token 加密儲存模組
//!
//! 使用簡單對稱加密（XOR）搭配機器特定金鑰，確保 Bot Token 不以明文儲存於資料庫中。
//! 金鑰衍生自機器識別資訊（使用者名稱 + 固定 salt）的雜湊值。

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

/// 固定 salt，用於金鑰衍生
const KEY_SALT: &str = "stockenboard-notification-key-v1";

/// 從機器識別資訊衍生 32 位元組金鑰
///
/// 使用使用者名稱（或 fallback）結合固定 salt，透過簡易雜湊產生金鑰。
fn derive_key() -> [u8; 32] {
    let machine_id = get_machine_identifier();
    let input = format!("{}:{}", KEY_SALT, machine_id);
    simple_hash(&input)
}

/// 取得機器識別資訊
///
/// 嘗試讀取環境變數中的使用者名稱，若無法取得則使用 fallback 值。
fn get_machine_identifier() -> String {
    // 嘗試多種環境變數取得使用者名稱
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .or_else(|_| std::env::var("LOGNAME"))
        .unwrap_or_else(|_| "stockenboard-default-user".to_string())
}

/// 簡易雜湊函數（基於多輪 XOR 混合），產生 32 位元組輸出
///
/// 注意：這不是密碼學安全的雜湊函數，但足以滿足「不以明文儲存」的需求。
fn simple_hash(input: &str) -> [u8; 32] {
    let bytes = input.as_bytes();
    let mut hash = [0u8; 32];

    // 初始化：將輸入分散到 hash 陣列
    for (i, &b) in bytes.iter().enumerate() {
        hash[i % 32] ^= b;
        hash[(i + 7) % 32] = hash[(i + 7) % 32].wrapping_add(b);
        hash[(i + 13) % 32] = hash[(i + 13) % 32].wrapping_mul(b.wrapping_add(1));
    }

    // 多輪混合以增加擴散性
    for round in 0..64u8 {
        for i in 0..32 {
            let prev = hash[(i + 31) % 32];
            let next = hash[(i + 1) % 32];
            hash[i] = hash[i]
                .wrapping_add(prev.rotate_left(3))
                ^ next.rotate_right(2)
                ^ round.wrapping_add(i as u8);
        }
    }

    hash
}

/// 加密 token
///
/// 將明文 token 以 XOR 加密後，回傳 base64 編碼的密文。
///
/// # 範例
/// ```
/// use stockenboard_lib::notifications::crypto::{encrypt_token, decrypt_token};
/// let encrypted = encrypt_token("my-secret-token").unwrap();
/// let decrypted = decrypt_token(&encrypted).unwrap();
/// assert_eq!(decrypted, "my-secret-token");
/// ```
pub fn encrypt_token(plaintext: &str) -> Result<String, String> {
    if plaintext.is_empty() {
        return Err("Token 不可為空".to_string());
    }

    let key = derive_key();
    let plaintext_bytes = plaintext.as_bytes();
    let encrypted: Vec<u8> = plaintext_bytes
        .iter()
        .enumerate()
        .map(|(i, &b)| b ^ key[i % 32])
        .collect();

    Ok(BASE64.encode(&encrypted))
}

/// 解密 token
///
/// 將 base64 編碼的密文解密回明文 token。
///
/// # 範例
/// ```
/// use stockenboard_lib::notifications::crypto::{encrypt_token, decrypt_token};
/// let encrypted = encrypt_token("my-secret-token").unwrap();
/// let decrypted = decrypt_token(&encrypted).unwrap();
/// assert_eq!(decrypted, "my-secret-token");
/// ```
pub fn decrypt_token(ciphertext: &str) -> Result<String, String> {
    if ciphertext.is_empty() {
        return Err("密文不可為空".to_string());
    }

    let encrypted_bytes = BASE64
        .decode(ciphertext)
        .map_err(|e| format!("Base64 解碼失敗: {}", e))?;

    let key = derive_key();
    let decrypted: Vec<u8> = encrypted_bytes
        .iter()
        .enumerate()
        .map(|(i, &b)| b ^ key[i % 32])
        .collect();

    String::from_utf8(decrypted).map_err(|e| format!("UTF-8 解碼失敗: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let token = "123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11";
        let encrypted = encrypt_token(token).unwrap();
        let decrypted = decrypt_token(&encrypted).unwrap();
        assert_eq!(decrypted, token);
    }

    #[test]
    fn test_encrypted_differs_from_plaintext() {
        let token = "123456:ABC-DEF1234ghIkl-zyx57W2v1u123ew11";
        let encrypted = encrypt_token(token).unwrap();
        assert_ne!(encrypted, token);
    }

    #[test]
    fn test_empty_token_returns_error() {
        let result = encrypt_token("");
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_ciphertext_returns_error() {
        let result = decrypt_token("");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_base64_returns_error() {
        let result = decrypt_token("not-valid-base64!!!");
        assert!(result.is_err());
    }

    #[test]
    fn test_deterministic_encryption() {
        let token = "test-token-12345";
        let encrypted1 = encrypt_token(token).unwrap();
        let encrypted2 = encrypt_token(token).unwrap();
        // Same machine, same key → same ciphertext
        assert_eq!(encrypted1, encrypted2);
    }

    #[test]
    fn test_different_tokens_produce_different_ciphertexts() {
        let encrypted1 = encrypt_token("token-aaa").unwrap();
        let encrypted2 = encrypt_token("token-bbb").unwrap();
        assert_ne!(encrypted1, encrypted2);
    }

    #[test]
    fn test_key_derivation_is_deterministic() {
        let key1 = derive_key();
        let key2 = derive_key();
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_simple_hash_produces_32_bytes() {
        let hash = simple_hash("test input");
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn test_simple_hash_different_inputs() {
        let hash1 = simple_hash("input-a");
        let hash2 = simple_hash("input-b");
        assert_ne!(hash1, hash2);
    }
}
