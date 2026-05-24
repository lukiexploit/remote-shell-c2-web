use crate::win32;

pub struct HttpClient {
    session: win32::HANDLE,
}

#[derive(Debug)]
pub struct HttpResponse {
    pub status: u32,
    pub body: Vec<u8>,
}

impl HttpClient {
    pub fn new() -> Option<Self> {
        let mut session = 0;
        unsafe {
            session = win32::WinHttpOpen(
                std::ptr::null(),
                win32::WINHTTP_ACCESS_TYPE_DEFAULT_PROXY,
                std::ptr::null(),
                std::ptr::null(),
                0,
            );
            if session == 0 {
                session = win32::WinHttpOpen(
                    std::ptr::null(),
                    win32::WINHTTP_ACCESS_TYPE_NO_PROXY,
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                );
            }
        }
        if session == 0 {
            return None;
        }
        Some(Self { session })
    }

    pub fn request(
        &self,
        method: &str,
        url: &str,
        headers: &[(&str, &str)],
        body: Option<&[u8]>,
    ) -> Option<HttpResponse> {
        unsafe {
            let (is_https, host, port, path) = parse_url(url)?;
            let host_wide: Vec<u16> =
                host.encode_utf16().chain(std::iter::once(0)).collect();
            let path_wide: Vec<u16> =
                path.encode_utf16().chain(std::iter::once(0)).collect();
            let method_wide: Vec<u16> =
                method.encode_utf16().chain(std::iter::once(0)).collect();

            let connect = win32::WinHttpConnect(
                self.session,
                host_wide.as_ptr(),
                port,
                0,
            );
            if connect == 0 {
                return None;
            }

            let flags = if is_https {
                win32::WINHTTP_FLAG_SECURE
            } else {
                0
            };

            let request = win32::WinHttpOpenRequest(
                connect,
                method_wide.as_ptr(),
                path_wide.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                flags,
            );
            if request == 0 {
                win32::WinHttpCloseHandle(connect);
                return None;
            }

            if !headers.is_empty() || body.is_some() {
                let header_str = headers
                    .iter()
                    .map(|(k, v)| format!("{}: {}\r\n", k, v))
                    .collect::<String>();
                let header_wide: Vec<u16> =
                    header_str.encode_utf16().chain(std::iter::once(0)).collect();

                let (body_ptr, body_len) = match body {
                    Some(b) => (b.as_ptr() as *mut std::ffi::c_void, b.len() as u32),
                    None => (std::ptr::null_mut(), 0),
                };

                let result = win32::WinHttpSendRequest(
                    request,
                    header_wide.as_ptr(),
                    header_str.len() as u32,
                    body_ptr,
                    body_len,
                    body_len,
                    0,
                );
                if result == 0 {
                    win32::WinHttpCloseHandle(request);
                    win32::WinHttpCloseHandle(connect);
                    return None;
                }
            } else {
                let result = win32::WinHttpSendRequest(
                    request,
                    std::ptr::null(),
                    0,
                    std::ptr::null_mut(),
                    0,
                    0,
                    0,
                );
                if result == 0 {
                    win32::WinHttpCloseHandle(request);
                    win32::WinHttpCloseHandle(connect);
                    return None;
                }
            }

            if win32::WinHttpReceiveResponse(request, std::ptr::null_mut()) == 0 {
                win32::WinHttpCloseHandle(request);
                win32::WinHttpCloseHandle(connect);
                return None;
            }

            let mut status: u32 = 0;
            let mut status_len: u32 = std::mem::size_of::<u32>() as u32;
            win32::WinHttpQueryHeaders(
                request,
                win32::WINHTTP_QUERY_STATUS_CODE | win32::WINHTTP_QUERY_FLAG_NUMBER,
                std::ptr::null(),
                &mut status as *mut _ as *mut std::ffi::c_void,
                &mut status_len,
                std::ptr::null_mut(),
            );

            let mut body = Vec::new();
            let mut buf = [0u8; 8192];
            loop {
                let mut bytes_read: u32 = 0;
                if win32::WinHttpReadData(
                    request,
                    buf.as_mut_ptr() as *mut std::ffi::c_void,
                    buf.len() as u32,
                    &mut bytes_read,
                ) == 0
                {
                    break;
                }
                if bytes_read == 0 {
                    break;
                }
                body.extend_from_slice(&buf[..bytes_read as usize]);
            }

            win32::WinHttpCloseHandle(request);
            win32::WinHttpCloseHandle(connect);

            Some(HttpResponse { status, body })
        }
    }
}

impl Drop for HttpClient {
    fn drop(&mut self) {
        unsafe {
            win32::WinHttpCloseHandle(self.session);
        }
    }
}

fn parse_url(url: &str) -> Option<(bool, String, u16, String)> {
    let (is_https, rest) = if url.starts_with("https://") {
        (true, &url[8..])
    } else if url.starts_with("http://") {
        (false, &url[7..])
    } else {
        return None;
    };

    let (host_part, path_part) = rest.split_once('/').unwrap_or((rest, ""));
    let path = if path_part.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", path_part)
    };

    let (host, port) = if let Some((h, p)) = host_part.split_once(':') {
        let port: u16 = p.parse().ok()?;
        (h.to_string(), port)
    } else {
        let port = if is_https { 443 } else { 80 };
        (host_part.to_string(), port)
    };

    Some((is_https, host, port, path))
}
