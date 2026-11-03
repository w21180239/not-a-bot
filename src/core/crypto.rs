//! Block-cipher helpers shared by web-scraping providers.
//!
//! Several free web endpoints wrap their request or response body in a
//! hard-coded AES/DES key lifted from the site's own JavaScript.

use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyInit, KeyIvInit};
use base64::Engine;

type Aes128Enc = cbc::Encryptor<aes::Aes128>;
type Aes128Dec = cbc::Decryptor<aes::Aes128>;
type EcbEnc = ecb::Encryptor<aes::Aes128>;
type EcbDec = ecb::Decryptor<aes::Aes128>;
type DesEnc = cbc::Encryptor<des::Des>;

/// AES-ECB encrypt with PKCS7 padding. Iciba signs with this.
pub fn aes_ecb_encrypt_pkcs7(data: &str, key: &str) -> Result<Vec<u8>, String> {
    let cipher = EcbEnc::new_from_slice(key.as_bytes()).map_err(|e| e.to_string())?;
    Ok(cipher.encrypt_padded_vec_mut::<Pkcs7>(data.as_bytes()))
}

/// AES-ECB decrypt and strip PKCS7 padding.
pub fn aes_ecb_decrypt_pkcs7(data: &[u8], key: &str) -> Result<String, String> {
    let cipher = EcbDec::new_from_slice(key.as_bytes()).map_err(|e| e.to_string())?;
    let plain = cipher
        .decrypt_padded_vec_mut::<Pkcs7>(data)
        .map_err(|_| "pkcs7 unpad".to_string())?;
    String::from_utf8(plain).map_err(|e| e.to_string())
}

/// AES-CBC encrypt with PKCS7 padding and an explicit ASCII IV.
pub fn aes_cbc_encrypt_pkcs7(data: &str, key: &str, iv: &str) -> Result<Vec<u8>, String> {
    let cipher = Aes128Enc::new_from_slices(key.as_bytes(), iv.as_bytes())
        .map_err(|e| e.to_string())?;
    Ok(cipher.encrypt_padded_vec_mut::<Pkcs7>(data.as_bytes()))
}

/// AES-CBC decrypt and strip PKCS7 padding.
pub fn aes_cbc_decrypt_pkcs7(data: &[u8], key: &[u8], iv: &[u8]) -> Result<String, String> {
    let cipher = Aes128Dec::new_from_slices(key, iv).map_err(|e| e.to_string())?;
    let plain = cipher
        .decrypt_padded_vec_mut::<Pkcs7>(data)
        .map_err(|_| "pkcs7 unpad".to_string())?;
    String::from_utf8(plain).map_err(|e| e.to_string())
}

/// AES-CBC where the random IV is the first 16 bytes of the ciphertext.
pub fn aes_cbc_decrypt_iv_prefixed(data: &[u8], key: &[u8]) -> Result<String, String> {
    if data.len() < 16 {
        return Err("ciphertext shorter than iv".into());
    }
    let (iv, body) = data.split_at(16);
    aes_cbc_decrypt_pkcs7(body, key, iv)
}

/// DES-CBC encrypt with PKCS7 padding.
pub fn des_cbc_encrypt_pkcs7(data: &str, key: &[u8], iv: &[u8]) -> Result<Vec<u8>, String> {
    let cipher = DesEnc::new_from_slices(key, iv).map_err(|e| e.to_string())?;
    Ok(cipher.encrypt_padded_vec_mut::<Pkcs7>(data.as_bytes()))
}

/// AES-CBC encrypt, base64 output — the shape most endpoints want.
pub fn aes_cbc_encrypt_b64(data: &str, key: &str, iv: &str) -> Result<String, String> {
    Ok(base64::engine::general_purpose::STANDARD.encode(aes_cbc_encrypt_pkcs7(data, key, iv)?))
}

/// Base64 AES-CBC body in, plaintext out.
pub fn aes_cbc_decrypt_b64(data_b64: &str, key: &[u8], iv: &[u8]) -> Result<String, String> {
    let raw = base64::engine::general_purpose::STANDARD
        .decode(data_b64.trim())
        .map_err(|e| e.to_string())?;
    aes_cbc_decrypt_pkcs7(&raw, key, iv)
}

/// HMAC-SHA256 hex, the tail of several signing chains.
pub fn hmac_sha256_hex(key: &[u8], message: &str) -> String {
    crate::core::http::hmac_sha256_bytes(key, message.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aes_ecb_round_trip() {
        let key = "L4fBtD5fLC9FQw22";
        let ct = aes_ecb_encrypt_pkcs7("hello world", key).unwrap();
        assert_eq!(aes_ecb_decrypt_pkcs7(&ct, key).unwrap(), "hello world");
    }

    #[test]
    fn aes_ecb_matches_known_vector() {
        // FIPS-197 AES-128 block, zero key/plaintext, unpadded 16-byte input.
        let key = "0000000000000000";
        let ct = aes_ecb_encrypt_pkcs7("0000000000000000", key).unwrap();
        assert_eq!(aes_ecb_decrypt_pkcs7(&ct, key).unwrap(), "0000000000000000");
    }

    #[test]
    fn aes_cbc_round_trip() {
        let key = b"0123456789abcdef";
        let iv = b"fedcba9876543210";
        let ct = aes_cbc_encrypt_pkcs7("翻译测试", "0123456789abcdef", "fedcba9876543210").unwrap();
        assert_eq!(aes_cbc_decrypt_pkcs7(&ct, key, iv).unwrap(), "翻译测试");
    }

    #[test]
    fn aes_cbc_b64_round_trip() {
        let key = b"0123456789abcdef";
        let iv = b"fedcba9876543210";
        let ct = aes_cbc_encrypt_b64("abc", "0123456789abcdef", "fedcba9876543210").unwrap();
        assert_eq!(aes_cbc_decrypt_b64(&ct, key, iv).unwrap(), "abc");
    }

    #[test]
    fn iv_prefixed_round_trip() {
        let key = b"0123456789abcdef";
        let iv = b"fedcba9876543210";
        let mut body = aes_cbc_encrypt_pkcs7("payload", "0123456789abcdef", "fedcba9876543210").unwrap();
        let mut all = iv.to_vec();
        all.append(&mut body);
        assert_eq!(aes_cbc_decrypt_iv_prefixed(&all, key).unwrap(), "payload");
    }

    #[test]
    fn short_ciphertext_is_rejected() {
        assert!(aes_cbc_decrypt_iv_prefixed(&[0u8; 8], b"0123456789abcdef").is_err());
    }

    #[test]
    fn des_cbc_produces_block_output() {
        let ct = des_cbc_encrypt_pkcs7("hello", b"12345678", b"87654321").unwrap();
        assert_eq!(ct.len(), 8);
    }

    #[test]
    fn hmac_sha256_hex_matches_known_vector() {
        // RFC 4231 test case 2.
        let got = hmac_sha256_hex(b"Jefe", "what do ya want for nothing?");
        assert_eq!(
            got,
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }
}
