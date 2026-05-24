use aes::Aes256;
use base64::Engine;
use base64::engine::general_purpose;
use cbc::{Decryptor, Encryptor};
use cipher::{
    block_padding::Pkcs7, BlockDecryptMut, BlockEncryptMut, KeyIvInit,
};
use rand::Rng;

type Aes256CbcEnc = Encryptor<Aes256>;
type Aes256CbcDec = Decryptor<Aes256>;

pub fn decrypt_key(hex_key: &str) -> [u8; 32] {
    let mut key = [0u8; 32];
    let decoded = hex::decode(hex_key).expect("invalid AES key hex");
    key.copy_from_slice(&decoded);
    key
}

pub fn encrypt(plaintext: &[u8], key: &[u8; 32]) -> Vec<u8> {
    let iv: [u8; 16] = rand::thread_rng().gen();
    let mut buf = vec![0u8; plaintext.len() + 32];
    buf[..plaintext.len()].copy_from_slice(plaintext);
    let ct = Aes256CbcEnc::new(key.into(), &iv.into())
        .encrypt_padded_mut::<Pkcs7>(&mut buf, plaintext.len())
        .expect("encrypt failed");
    let mut out = iv.to_vec();
    out.extend_from_slice(ct);
    out
}

pub fn decrypt(data: &[u8], key: &[u8; 32]) -> Option<Vec<u8>> {
    if data.len() < 16 {
        return None;
    }
    let (iv, ct) = data.split_at(16);
    let mut buf = ct.to_vec();
    let pt = Aes256CbcDec::new(key.into(), iv.into())
        .decrypt_padded_mut::<Pkcs7>(&mut buf)
        .ok()?;
    Some(pt.to_vec())
}

fn engine() -> base64::engine::GeneralPurpose {
    general_purpose::STANDARD
}

pub fn encrypt_b64(plaintext: &[u8], key: &[u8; 32]) -> String {
    engine().encode(&encrypt(plaintext, key))
}

pub fn decrypt_b64(ciphertext_b64: &str, key: &[u8; 32]) -> Option<Vec<u8>> {
    let raw = engine().decode(ciphertext_b64).ok()?;
    decrypt(&raw, key)
}
