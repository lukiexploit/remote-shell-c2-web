// Win32 FFI — raw extern declarations, zero external bindings
#![allow(non_snake_case, non_camel_case_types, dead_code)]

use std::ffi::c_void;

pub type BOOL = i32;
pub type DWORD = u32;
pub type HANDLE = isize;
pub type HINSTANCE = isize;
pub type HMODULE = isize;
pub type LPVOID = *mut c_void;
pub type LPCVOID = *const c_void;
pub type LPCWSTR = *const u16;
pub type LPWSTR = *mut u16;

pub const TRUE: BOOL = 1;
pub const FALSE: BOOL = 0;
pub const NULL: HANDLE = 0;
pub const INVALID_HANDLE_VALUE: HANDLE = -1;

pub const DLL_PROCESS_ATTACH: DWORD = 1;
pub const DLL_PROCESS_DETACH: DWORD = 0;
pub const DLL_THREAD_ATTACH: DWORD = 2;
pub const DLL_THREAD_DETACH: DWORD = 3;

// --- Kernel32 ---

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SECURITY_ATTRIBUTES {
    pub nLength: DWORD,
    pub lpSecurityDescriptor: LPVOID,
    pub bInheritHandle: BOOL,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct STARTUPINFOW {
    pub cb: DWORD,
    pub lpReserved: LPWSTR,
    pub lpDesktop: LPWSTR,
    pub lpTitle: LPWSTR,
    pub dwX: DWORD,
    pub dwY: DWORD,
    pub dwXSize: DWORD,
    pub dwYSize: DWORD,
    pub dwXCountChars: DWORD,
    pub dwYCountChars: DWORD,
    pub dwFillAttribute: DWORD,
    pub dwFlags: DWORD,
    pub wShowWindow: u16,
    pub cbReserved2: u16,
    pub lpReserved2: *mut u8,
    pub hStdInput: HANDLE,
    pub hStdOutput: HANDLE,
    pub hStdError: HANDLE,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PROCESS_INFORMATION {
    pub hProcess: HANDLE,
    pub hThread: HANDLE,
    pub dwProcessId: DWORD,
    pub dwThreadId: DWORD,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SYSTEM_INFO {
    pub wProcessorArchitecture: u16,
    pub wReserved: u16,
    pub dwPageSize: DWORD,
    pub lpMinimumApplicationAddress: LPVOID,
    pub lpMaximumApplicationAddress: LPVOID,
    pub dwActiveProcessorMask: usize,
    pub dwNumberOfProcessors: DWORD,
    pub dwProcessorType: DWORD,
    pub dwAllocationGranularity: DWORD,
    pub wProcessorLevel: u16,
    pub wProcessorRevision: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MEMORYSTATUSEX {
    pub dwLength: DWORD,
    pub dwMemoryLoad: DWORD,
    pub ullTotalPhys: u64,
    pub ullAvailPhys: u64,
    pub ullTotalPageFile: u64,
    pub ullAvailPageFile: u64,
    pub ullTotalVirtual: u64,
    pub ullAvailVirtual: u64,
    pub ullAvailExtendedVirtual: u64,
}

pub const STARTF_USESTDHANDLES: DWORD = 0x00000100;
pub const HANDLE_FLAG_INHERIT: DWORD = 0x00000001;
pub const STD_INPUT_HANDLE: DWORD = 0xFFFFFFF6u32;
pub const WAIT_OBJECT_0: DWORD = 0;
pub const WAIT_TIMEOUT: DWORD = 258;

#[link(name = "kernel32")]
extern "system" {
    pub fn CloseHandle(hObject: HANDLE) -> BOOL;
    pub fn SetHandleInformation(hObject: HANDLE, dwMask: DWORD, dwFlags: DWORD) -> BOOL;
    pub fn GetStdHandle(nStdHandle: DWORD) -> HANDLE;

    pub fn CreatePipe(
        hReadPipe: *mut HANDLE,
        hWritePipe: *mut HANDLE,
        lpPipeAttributes: *const SECURITY_ATTRIBUTES,
        nSize: DWORD,
    ) -> BOOL;

    pub fn CreateProcessW(
        lpApplicationName: LPCWSTR,
        lpCommandLine: LPWSTR,
        lpProcessAttributes: LPVOID,
        lpThreadAttributes: LPVOID,
        bInheritHandles: BOOL,
        dwCreationFlags: DWORD,
        lpEnvironment: LPVOID,
        lpCurrentDirectory: LPCWSTR,
        lpStartupInfo: *const STARTUPINFOW,
        lpProcessInformation: *mut PROCESS_INFORMATION,
    ) -> BOOL;

    pub fn ReadFile(
        hFile: HANDLE,
        lpBuffer: LPVOID,
        nNumberOfBytesToRead: DWORD,
        lpNumberOfBytesRead: *mut DWORD,
        lpOverlapped: LPVOID,
    ) -> BOOL;

    pub fn WriteFile(
        hFile: HANDLE,
        lpBuffer: LPCVOID,
        nNumberOfBytesToWrite: DWORD,
        lpNumberOfBytesWritten: *mut DWORD,
        lpOverlapped: LPVOID,
    ) -> BOOL;

    pub fn WaitForSingleObject(hHandle: HANDLE, dwMilliseconds: DWORD) -> DWORD;
    pub fn GetExitCodeProcess(hProcess: HANDLE, lpExitCode: *mut DWORD) -> BOOL;
    pub fn IsDebuggerPresent() -> BOOL;
    pub fn GetNativeSystemInfo(lpSystemInfo: *mut SYSTEM_INFO);
    pub fn GlobalMemoryStatusEx(lpBuffer: *mut MEMORYSTATUSEX) -> BOOL;
}

// --- WinHTTP ---

#[link(name = "winhttp")]
extern "system" {
    pub fn WinHttpOpen(
        pszAgentW: LPCWSTR,
        dwAccessType: DWORD,
        pszProxyW: LPCWSTR,
        pszProxyBypassW: LPCWSTR,
        dwFlags: DWORD,
    ) -> HANDLE;

    pub fn WinHttpCloseHandle(hInternet: HANDLE) -> BOOL;

    pub fn WinHttpConnect(
        hSession: HANDLE,
        pswzServerName: LPCWSTR,
        nServerPort: u16,
        dwReserved: DWORD,
    ) -> HANDLE;

    pub fn WinHttpOpenRequest(
        hConnect: HANDLE,
        pwszVerb: LPCWSTR,
        pwszObjectName: LPCWSTR,
        pwszVersion: LPCWSTR,
        pwszReferrer: LPCWSTR,
        ppwszAcceptTypes: *const LPCWSTR,
        dwFlags: DWORD,
    ) -> HANDLE;

    pub fn WinHttpSendRequest(
        hRequest: HANDLE,
        lpszHeaders: LPCWSTR,
        dwHeadersLength: DWORD,
        lpOptional: LPVOID,
        dwOptionalLength: DWORD,
        dwTotalLength: DWORD,
        dwContext: usize,
    ) -> BOOL;

    pub fn WinHttpReceiveResponse(hRequest: HANDLE, lpvReserved: LPVOID) -> BOOL;

    pub fn WinHttpQueryHeaders(
        hRequest: HANDLE,
        dwInfoLevel: DWORD,
        pwszName: LPCWSTR,
        lpBuffer: LPVOID,
        lpdwBufferLength: *mut DWORD,
        lpdwIndex: *mut DWORD,
    ) -> BOOL;

    pub fn WinHttpReadData(
        hRequest: HANDLE,
        lpBuffer: LPVOID,
        dwNumberOfBytesToRead: DWORD,
        lpdwNumberOfBytesRead: *mut DWORD,
    ) -> BOOL;
}

pub const WINHTTP_ACCESS_TYPE_DEFAULT_PROXY: DWORD = 0;
pub const WINHTTP_ACCESS_TYPE_NO_PROXY: DWORD = 1;
pub const WINHTTP_FLAG_SECURE: DWORD = 0x00800000;
pub const WINHTTP_QUERY_FLAG_NUMBER: DWORD = 0x20000000;
pub const WINHTTP_QUERY_STATUS_CODE: DWORD = 19;

// --- GDI32 ---

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BITMAPINFOHEADER {
    pub biSize: DWORD,
    pub biWidth: i32,
    pub biHeight: i32,
    pub biPlanes: u16,
    pub biBitCount: u16,
    pub biCompression: DWORD,
    pub biSizeImage: DWORD,
    pub biXPelsPerMeter: i32,
    pub biYPelsPerMeter: i32,
    pub biClrUsed: DWORD,
    pub biClrImportant: DWORD,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BITMAPINFO {
    pub bmiHeader: BITMAPINFOHEADER,
    pub bmiColors: [u32; 1],
}

pub const SRCCOPY: DWORD = 0x00CC0020;
pub const BI_RGB: DWORD = 0;
pub const DIB_RGB_COLORS: DWORD = 0;
pub const DESKTOPHORZRES: i32 = 118;
pub const DESKTOPVERTRES: i32 = 117;

#[link(name = "gdi32")]
extern "system" {
    pub fn CreateCompatibleDC(hdc: HANDLE) -> HANDLE;
    pub fn DeleteDC(hdc: HANDLE) -> BOOL;
    pub fn CreateCompatibleBitmap(hdc: HANDLE, cx: i32, cy: i32) -> HANDLE;
    pub fn DeleteObject(ho: HANDLE) -> BOOL;
    pub fn SelectObject(hdc: HANDLE, h: HANDLE) -> HANDLE;
    pub fn BitBlt(
        hdc: HANDLE,
        x: i32, y: i32, cx: i32, cy: i32,
        hdcSrc: HANDLE,
        x1: i32, y1: i32,
        rop: DWORD,
    ) -> BOOL;
    pub fn GetDeviceCaps(hdc: HANDLE, index: i32) -> i32;
    pub fn GetDIBits(
        hdc: HANDLE,
        hbm: HANDLE,
        start: u32,
        cLines: u32,
        lpvBits: LPVOID,
        lpbmi: *mut BITMAPINFO,
        usage: DWORD,
    ) -> i32;
}

// --- User32 ---

#[link(name = "user32")]
extern "system" {
    pub fn GetDC(hWnd: HANDLE) -> HANDLE;
    pub fn ReleaseDC(hWnd: HANDLE, hDC: HANDLE) -> i32;
}
