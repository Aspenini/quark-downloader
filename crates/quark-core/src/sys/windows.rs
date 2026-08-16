use std::ffi::{OsStr, OsString};
use std::io;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::atomic::{AtomicU8, Ordering};

pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;
pub const CREATE_SUSPENDED: u32 = 0x0000_0004;
pub const CREATE_UNICODE_ENVIRONMENT: u32 = 0x0000_0400;
pub const STARTF_USESTDHANDLES: u32 = 0x0000_0100;
pub const HANDLE_FLAG_INHERIT: u32 = 0x0000_0001;
pub const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: u32 = 0x0000_2000;
pub const INFINITE: u32 = 0xFFFF_FFFF;
pub const STD_OUTPUT_HANDLE: u32 = 0xFFFF_FFF5; // -11 as u32
pub const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;
const INVALID_HANDLE_VALUE: isize = -1;
const CP_UTF8: u32 = 65001;
const WINHTTP_ACCESS_TYPE_DEFAULT_PROXY: u32 = 0;
const WINHTTP_FLAG_SECURE: u32 = 0x0080_0000;
const WINHTTP_QUERY_STATUS_CODE: u32 = 19;
const WINHTTP_QUERY_LOCATION: u32 = 33;
const WINHTTP_QUERY_FLAG_NUMBER: u32 = 0x2000_0000;
const BCRYPT_HASH_LENGTH: u32 = 0x0000_0001;
const BCRYPT_ALG_HANDLE_HMAC_FLAG: u32 = 0; // unused, keep silence

type Handle = *mut core::ffi::c_void;
type Bool = i32;
type Dword = u32;

#[repr(C)]
struct SecurityAttributes {
    n_length: Dword,
    lp_security_descriptor: *mut core::ffi::c_void,
    b_inherit_handle: Bool,
}

#[repr(C)]
struct StartupInfoW {
    cb: Dword,
    reserved: *mut u16,
    desktop: *mut u16,
    title: *mut u16,
    dw_x: Dword,
    dw_y: Dword,
    dw_x_size: Dword,
    dw_y_size: Dword,
    dw_x_count_chars: Dword,
    dw_y_count_chars: Dword,
    dw_fill_attribute: Dword,
    dw_flags: Dword,
    w_show_window: u16,
    cb_reserved2: u16,
    lp_reserved2: *mut u8,
    h_std_input: Handle,
    h_std_output: Handle,
    h_std_error: Handle,
}

#[repr(C)]
struct ProcessInformation {
    h_process: Handle,
    h_thread: Handle,
    dw_process_id: Dword,
    dw_thread_id: Dword,
}

// JOBOBJECT_EXTENDED_LIMIT_INFORMATION (class 9) — only LimitFlags is set.
#[repr(C)]
struct IoCounters {
    read_op: u64,
    write_op: u64,
    other_op: u64,
    read_xfer: u64,
    write_xfer: u64,
    other_xfer: u64,
}

#[repr(C)]
struct JobObjectBasicLimitInformation {
    per_process_user_time_limit: i64,
    per_job_user_time_limit: i64,
    limit_flags: Dword,
    minimum_working_set_size: usize,
    maximum_working_set_size: usize,
    active_process_limit: Dword,
    affinity: usize,
    priority_class: Dword,
    scheduling_class: Dword,
}

#[repr(C)]
struct JobObjectExtendedLimit {
    basic_limit_information: JobObjectBasicLimitInformation,
    io_info: IoCounters,
    process_memory_limit: usize,
    job_memory_limit: usize,
    peak_process_memory_used: usize,
    peak_job_memory_used: usize,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetStdHandle(nStdHandle: Dword) -> Handle;
    fn GetConsoleMode(hConsoleHandle: Handle, lpMode: *mut Dword) -> Bool;
    fn SetConsoleMode(hConsoleHandle: Handle, dwMode: Dword) -> Bool;
    fn CreatePipe(
        hReadPipe: *mut Handle,
        hWritePipe: *mut Handle,
        lpPipeAttributes: *mut SecurityAttributes,
        nSize: Dword,
    ) -> Bool;
    fn SetHandleInformation(hObject: Handle, dwMask: Dword, dwFlags: Dword) -> Bool;
    fn CreateProcessW(
        lpApplicationName: *const u16,
        lpCommandLine: *mut u16,
        lpProcessAttributes: *mut SecurityAttributes,
        lpThreadAttributes: *mut SecurityAttributes,
        bInheritHandles: Bool,
        dwCreationFlags: Dword,
        lpEnvironment: *mut core::ffi::c_void,
        lpCurrentDirectory: *const u16,
        lpStartupInfo: *mut StartupInfoW,
        lpProcessInformation: *mut ProcessInformation,
    ) -> Bool;
    fn CreateJobObjectW(lpJobAttributes: *mut SecurityAttributes, lpName: *const u16) -> Handle;
    fn SetInformationJobObject(
        hJob: Handle,
        jobObjectInformationClass: i32,
        lpJobObjectInformation: *const core::ffi::c_void,
        cbJobObjectInformationLength: Dword,
    ) -> Bool;
    fn AssignProcessToJobObject(hJob: Handle, hProcess: Handle) -> Bool;
    fn ResumeThread(hThread: Handle) -> Dword;
    fn WaitForSingleObject(hHandle: Handle, dwMilliseconds: Dword) -> Dword;
    fn GetExitCodeProcess(hProcess: Handle, lpExitCode: *mut Dword) -> Bool;
    fn TerminateJobObject(hJob: Handle, uExitCode: u32) -> Bool;
    fn CloseHandle(hObject: Handle) -> Bool;
    fn ReadFile(
        hFile: Handle,
        lpBuffer: *mut u8,
        nNumberOfBytesToRead: Dword,
        lpNumberOfBytesRead: *mut Dword,
        lpOverlapped: *mut core::ffi::c_void,
    ) -> Bool;
    fn SearchPathW(
        lpPath: *const u16,
        lpFileName: *const u16,
        lpExtension: *const u16,
        nBufferLength: Dword,
        lpBuffer: *mut u16,
        lpFilePart: *mut *mut u16,
    ) -> Dword;
}

#[link(name = "bcrypt")]
unsafe extern "system" {
    fn BCryptOpenAlgorithmProvider(
        phAlgorithm: *mut Handle,
        pszAlgId: *const u16,
        pszImplementation: *const u16,
        dwFlags: Dword,
    ) -> i32;
    fn BCryptGetProperty(
        hObject: Handle,
        pszProperty: *const u16,
        pbOutput: *mut u8,
        cbOutput: Dword,
        pcbResult: *mut Dword,
        dwFlags: Dword,
    ) -> i32;
    fn BCryptCreateHash(
        hAlgorithm: Handle,
        phHash: *mut Handle,
        pbHashObject: *mut u8,
        cbHashObject: Dword,
        pbSecret: *mut u8,
        cbSecret: Dword,
        dwFlags: Dword,
    ) -> i32;
    fn BCryptHashData(hHash: Handle, pbInput: *const u8, cbInput: Dword, dwFlags: Dword) -> i32;
    fn BCryptFinishHash(hHash: Handle, pbOutput: *mut u8, cbOutput: Dword, dwFlags: Dword) -> i32;
    fn BCryptDestroyHash(hHash: Handle) -> i32;
    fn BCryptCloseAlgorithmProvider(hAlgorithm: Handle, dwFlags: Dword) -> i32;
}

#[link(name = "winhttp")]
unsafe extern "system" {
    fn WinHttpOpen(
        pszAgentW: *const u16,
        dwAccessType: Dword,
        pszProxyW: *const u16,
        pszProxyBypassW: *const u16,
        dwFlags: Dword,
    ) -> Handle;
    fn WinHttpConnect(
        hSession: Handle,
        pswzServerName: *const u16,
        nServerPort: u16,
        dwReserved: Dword,
    ) -> Handle;
    fn WinHttpOpenRequest(
        hConnect: Handle,
        pwszVerb: *const u16,
        pwszObjectName: *const u16,
        pwszVersion: *const u16,
        pwszReferrer: *const u16,
        ppwszAcceptTypes: *mut *const u16,
        dwFlags: Dword,
    ) -> Handle;
    fn WinHttpSendRequest(
        hRequest: Handle,
        lpszHeaders: *const u16,
        dwHeadersLength: Dword,
        lpOptional: *mut core::ffi::c_void,
        dwOptionalLength: Dword,
        dwTotalLength: Dword,
        dwContext: usize,
    ) -> Bool;
    fn WinHttpReceiveResponse(hRequest: Handle, lpReserved: *mut core::ffi::c_void) -> Bool;
    fn WinHttpQueryHeaders(
        hRequest: Handle,
        dwInfoLevel: Dword,
        pwszName: *const u16,
        lpBuffer: *mut core::ffi::c_void,
        lpdwBufferLength: *mut Dword,
        lpdwIndex: *mut Dword,
    ) -> Bool;
    fn WinHttpQueryDataAvailable(hRequest: Handle, lpdwNumberOfBytesAvailable: *mut Dword) -> Bool;
    fn WinHttpReadData(
        hRequest: Handle,
        lpBuffer: *mut core::ffi::c_void,
        dwNumberOfBytesToRead: Dword,
        lpdwNumberOfBytesRead: *mut Dword,
    ) -> Bool;
    fn WinHttpCloseHandle(hInternet: Handle) -> Bool;
}

fn wide(s: impl AsRef<OsStr>) -> Vec<u16> {
    s.as_ref().encode_wide().chain(std::iter::once(0)).collect()
}

pub fn enable_virtual_terminal(tried: &AtomicU8) {
    if tried.swap(1, Ordering::Relaxed) == 1 {
        return;
    }
    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        if handle.is_null() || handle as isize == INVALID_HANDLE_VALUE {
            return;
        }
        let mut mode: Dword = 0;
        if GetConsoleMode(handle, &mut mode) == 0 {
            return;
        }
        let _ = SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
    }
}

pub fn which(name: &str) -> Option<PathBuf> {
    let file = if name.ends_with(".exe") {
        name.to_string()
    } else {
        format!("{name}.exe")
    };
    let wname = wide(&file);
    let mut buf = vec![0u16; 1024];
    unsafe {
        let n = SearchPathW(
            ptr::null(),
            wname.as_ptr(),
            ptr::null(),
            buf.len() as Dword,
            buf.as_mut_ptr(),
            ptr::null_mut(),
        );
        if n == 0 || n as usize >= buf.len() {
            return None;
        }
        Some(PathBuf::from(OsString::from_wide(&buf[..n as usize])))
    }
}

pub fn sha256_hex(path: &Path) -> io::Result<String> {
    let data = std::fs::read(path)?;
    let alg = wide("SHA256");
    let prop = wide("ObjectLength");
    unsafe {
        let mut h_alg: Handle = ptr::null_mut();
        if BCryptOpenAlgorithmProvider(&mut h_alg, alg.as_ptr(), ptr::null(), 0) != 0 {
            return Err(io::Error::other("BCryptOpenAlgorithmProvider failed"));
        }
        let mut obj_len: Dword = 0;
        let mut cb: Dword = 0;
        if BCryptGetProperty(
            h_alg,
            prop.as_ptr(),
            (&raw mut obj_len).cast(),
            size_of::<Dword>() as Dword,
            &mut cb,
            0,
        ) != 0
        {
            BCryptCloseAlgorithmProvider(h_alg, 0);
            return Err(io::Error::other("BCryptGetProperty failed"));
        }
        let mut obj = vec![0u8; obj_len as usize];
        let mut h_hash: Handle = ptr::null_mut();
        if BCryptCreateHash(
            h_alg,
            &mut h_hash,
            obj.as_mut_ptr(),
            obj_len,
            ptr::null_mut(),
            0,
            0,
        ) != 0
        {
            BCryptCloseAlgorithmProvider(h_alg, 0);
            return Err(io::Error::other("BCryptCreateHash failed"));
        }
        if BCryptHashData(h_hash, data.as_ptr(), data.len() as Dword, 0) != 0 {
            BCryptDestroyHash(h_hash);
            BCryptCloseAlgorithmProvider(h_alg, 0);
            return Err(io::Error::other("BCryptHashData failed"));
        }
        let mut hash = [0u8; 32];
        if BCryptFinishHash(h_hash, hash.as_mut_ptr(), 32, 0) != 0 {
            BCryptDestroyHash(h_hash);
            BCryptCloseAlgorithmProvider(h_alg, 0);
            return Err(io::Error::other("BCryptFinishHash failed"));
        }
        BCryptDestroyHash(h_hash);
        BCryptCloseAlgorithmProvider(h_alg, 0);
        Ok(hash.iter().map(|b| format!("{b:02x}")).collect())
    }
}

struct WinHttpUrl {
    host: String,
    path: String,
    secure: bool,
    port: u16,
}

fn parse_http_url(url: &str) -> io::Result<WinHttpUrl> {
    let (secure, rest) = if let Some(r) = url.strip_prefix("https://") {
        (true, r)
    } else if let Some(r) = url.strip_prefix("http://") {
        (false, r)
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "not an http url",
        ));
    };
    let (hostport, path) = match rest.split_once('/') {
        Some((h, p)) => (h, format!("/{p}")),
        None => (rest, "/".into()),
    };
    let (host, port) = if let Some((h, p)) = hostport.split_once(':') {
        (
            h.to_string(),
            p.parse::<u16>()
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "bad port"))?,
        )
    } else {
        (hostport.to_string(), if secure { 443 } else { 80 })
    };
    Ok(WinHttpUrl {
        host,
        path,
        secure,
        port,
    })
}

fn winhttp_request(url: &str, user_agent: &str) -> io::Result<(u32, Option<String>, Vec<u8>)> {
    let parsed = parse_http_url(url)?;
    let agent = wide(user_agent);
    let host = wide(&parsed.host);
    let path = wide(&parsed.path);
    let verb = wide("GET");
    unsafe {
        let session = WinHttpOpen(
            agent.as_ptr(),
            WINHTTP_ACCESS_TYPE_DEFAULT_PROXY,
            ptr::null(),
            ptr::null(),
            0,
        );
        if session.is_null() {
            return Err(io::Error::other("WinHttpOpen failed"));
        }
        let connect = WinHttpConnect(session, host.as_ptr(), parsed.port, 0);
        if connect.is_null() {
            WinHttpCloseHandle(session);
            return Err(io::Error::other("WinHttpConnect failed"));
        }
        let flags = if parsed.secure {
            WINHTTP_FLAG_SECURE
        } else {
            0
        };
        let request = WinHttpOpenRequest(
            connect,
            verb.as_ptr(),
            path.as_ptr(),
            ptr::null(),
            ptr::null(),
            ptr::null_mut(),
            flags,
        );
        if request.is_null() {
            WinHttpCloseHandle(connect);
            WinHttpCloseHandle(session);
            return Err(io::Error::other("WinHttpOpenRequest failed"));
        }
        if WinHttpSendRequest(request, ptr::null(), 0, ptr::null_mut(), 0, 0, 0) == 0
            || WinHttpReceiveResponse(request, ptr::null_mut()) == 0
        {
            WinHttpCloseHandle(request);
            WinHttpCloseHandle(connect);
            WinHttpCloseHandle(session);
            return Err(io::Error::other("WinHttp request failed"));
        }
        let mut status: Dword = 0;
        let mut status_len = size_of::<Dword>() as Dword;
        let mut index: Dword = 0;
        let _ = WinHttpQueryHeaders(
            request,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            ptr::null(),
            (&raw mut status).cast(),
            &mut status_len,
            &mut index,
        );
        let mut location = None;
        if matches!(status, 301 | 302 | 303 | 307 | 308) {
            let mut loc_len: Dword = 0;
            index = 0;
            WinHttpQueryHeaders(
                request,
                WINHTTP_QUERY_LOCATION,
                ptr::null(),
                ptr::null_mut(),
                &mut loc_len,
                &mut index,
            );
            if loc_len > 0 {
                let mut buf = vec![0u16; (loc_len as usize / 2) + 1];
                index = 0;
                if WinHttpQueryHeaders(
                    request,
                    WINHTTP_QUERY_LOCATION,
                    ptr::null(),
                    buf.as_mut_ptr().cast(),
                    &mut loc_len,
                    &mut index,
                ) != 0
                {
                    let n = (loc_len as usize) / 2;
                    location = Some(
                        OsString::from_wide(&buf[..n])
                            .to_string_lossy()
                            .into_owned(),
                    );
                }
            }
        }
        let mut body = Vec::new();
        loop {
            let mut available: Dword = 0;
            if WinHttpQueryDataAvailable(request, &mut available) == 0 {
                break;
            }
            if available == 0 {
                break;
            }
            let mut chunk = vec![0u8; available as usize];
            let mut read: Dword = 0;
            if WinHttpReadData(request, chunk.as_mut_ptr().cast(), available, &mut read) == 0 {
                break;
            }
            body.extend_from_slice(&chunk[..read as usize]);
        }
        WinHttpCloseHandle(request);
        WinHttpCloseHandle(connect);
        WinHttpCloseHandle(session);
        Ok((status, location, body))
    }
}

fn follow_url(current: &str, location: &str) -> String {
    if location.starts_with("http://") || location.starts_with("https://") {
        location.to_string()
    } else if let Ok(base) = parse_http_url(current) {
        let scheme = if base.secure { "https" } else { "http" };
        if let Some(rest) = location.strip_prefix('/') {
            format!("{scheme}://{}:{}/{}", base.host, base.port, rest)
        } else {
            format!("{scheme}://{}:{}/{}", base.host, base.port, location)
        }
    } else {
        location.to_string()
    }
}

pub fn fetch_body(url: &str, user_agent: &str) -> io::Result<String> {
    let mut current = url.to_string();
    for _ in 0..5 {
        let (status, location, body) = winhttp_request(&current, user_agent)?;
        match status {
            200 => {
                if body.is_empty() {
                    return Err(io::Error::other(format!("Empty response from {current}")));
                }
                return String::from_utf8(body)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e));
            }
            301 | 302 | 303 | 307 | 308 => {
                let loc =
                    location.ok_or_else(|| io::Error::other("Redirect without Location header"))?;
                current = follow_url(&current, &loc);
            }
            other => {
                return Err(io::Error::other(format!("HTTP request failed: {other}")));
            }
        }
    }
    Err(io::Error::other("Too many redirects"))
}

pub fn download_file(url: &str, dest: &Path, user_agent: &str) -> io::Result<()> {
    let part = dest.with_file_name(format!(
        "{}.part",
        dest.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("download")
    ));
    let _ = std::fs::remove_file(&part);
    let _ = std::fs::remove_file(dest);
    let mut current = url.to_string();
    let mut written = false;
    let result = (|| {
        for _ in 0..5 {
            let (status, location, body) = winhttp_request(&current, user_agent)?;
            match status {
                200 => {
                    std::fs::write(&part, body)?;
                    std::fs::rename(&part, dest).or_else(|_| {
                        std::fs::copy(&part, dest)?;
                        std::fs::remove_file(&part)
                    })?;
                    written = true;
                    return Ok(());
                }
                301 | 302 | 303 | 307 | 308 => {
                    let loc = location
                        .ok_or_else(|| io::Error::other("Redirect without Location header"))?;
                    current = follow_url(&current, &loc);
                }
                other => {
                    return Err(io::Error::other(format!("HTTP request failed: {other}")));
                }
            }
        }
        Err(io::Error::other("Too many redirects"))
    })();
    if !written {
        let _ = std::fs::remove_file(&part);
    }
    result
}

pub struct HiddenProcess {
    process: Handle,
    job: Handle,
    stdout: Handle,
    stderr: Handle,
}

unsafe impl Send for HiddenProcess {}

impl HiddenProcess {
    pub fn spawn(command: &str, args: &[String]) -> io::Result<Self> {
        let cmdline =
            quote_windows(std::iter::once(command).chain(args.iter().map(String::as_str)));
        let mut cmd_wide = wide(&cmdline);
        let mut sa = SecurityAttributes {
            n_length: size_of::<SecurityAttributes>() as Dword,
            lp_security_descriptor: ptr::null_mut(),
            b_inherit_handle: 1,
        };
        let mut stdout_r: Handle = ptr::null_mut();
        let mut stdout_w: Handle = ptr::null_mut();
        let mut stderr_r: Handle = ptr::null_mut();
        let mut stderr_w: Handle = ptr::null_mut();
        unsafe {
            if CreatePipe(&mut stdout_r, &mut stdout_w, &mut sa, 0) == 0
                || CreatePipe(&mut stderr_r, &mut stderr_w, &mut sa, 0) == 0
            {
                return Err(io::Error::last_os_error());
            }
            SetHandleInformation(stdout_r, HANDLE_FLAG_INHERIT, 0);
            SetHandleInformation(stderr_r, HANDLE_FLAG_INHERIT, 0);
            let mut si: StartupInfoW = std::mem::zeroed();
            si.cb = size_of::<StartupInfoW>() as Dword;
            si.dw_flags = STARTF_USESTDHANDLES;
            si.h_std_output = stdout_w;
            si.h_std_error = stderr_w;
            let mut pi: ProcessInformation = std::mem::zeroed();
            let job = CreateJobObjectW(ptr::null_mut(), ptr::null());
            if !job.is_null() {
                let mut info: JobObjectExtendedLimit = std::mem::zeroed();
                info.basic_limit_information.limit_flags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                let _ = SetInformationJobObject(
                    job,
                    9, // JobObjectExtendedLimitInformation
                    (&raw const info).cast(),
                    size_of::<JobObjectExtendedLimit>() as Dword,
                );
            }
            let flags = CREATE_NO_WINDOW | CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT;
            if CreateProcessW(
                ptr::null(),
                cmd_wide.as_mut_ptr(),
                ptr::null_mut(),
                ptr::null_mut(),
                1,
                flags,
                ptr::null_mut(),
                ptr::null(),
                &mut si,
                &mut pi,
            ) == 0
            {
                CloseHandle(stdout_r);
                CloseHandle(stdout_w);
                CloseHandle(stderr_r);
                CloseHandle(stderr_w);
                if !job.is_null() {
                    CloseHandle(job);
                }
                return Err(io::Error::last_os_error());
            }
            if !job.is_null() {
                let _ = AssignProcessToJobObject(job, pi.h_process);
            }
            ResumeThread(pi.h_thread);
            CloseHandle(pi.h_thread);
            CloseHandle(stdout_w);
            CloseHandle(stderr_w);
            let _ = cmdline;
            Ok(Self {
                process: pi.h_process,
                job,
                stdout: stdout_r,
                stderr: stderr_r,
            })
        }
    }

    pub fn stdout_handle(&self) -> Handle {
        self.stdout
    }

    pub fn stderr_handle(&self) -> Handle {
        self.stderr
    }

    pub fn wait_ms(&self, ms: u32) -> Option<u32> {
        unsafe {
            let r = WaitForSingleObject(self.process, ms);
            if r == 0 {
                let mut code: Dword = 1;
                GetExitCodeProcess(self.process, &mut code);
                Some(code)
            } else {
                None
            }
        }
    }

    pub fn wait(&self) -> u32 {
        self.wait_ms(INFINITE).unwrap_or(1)
    }

    pub fn terminate(&self) {
        unsafe {
            if !self.job.is_null() {
                let _ = TerminateJobObject(self.job, 1);
            }
        }
    }
}

impl Drop for HiddenProcess {
    fn drop(&mut self) {
        unsafe {
            if !self.stdout.is_null() {
                CloseHandle(self.stdout);
            }
            if !self.stderr.is_null() {
                CloseHandle(self.stderr);
            }
            if !self.process.is_null() {
                CloseHandle(self.process);
            }
            if !self.job.is_null() {
                CloseHandle(self.job);
            }
        }
    }
}

pub fn read_handle_lines(handle: Handle, mut on_line: impl FnMut(&str)) {
    let mut leftover = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        let mut read: Dword = 0;
        let ok = unsafe {
            ReadFile(
                handle,
                buf.as_mut_ptr(),
                buf.len() as Dword,
                &mut read,
                ptr::null_mut(),
            )
        };
        if ok == 0 || read == 0 {
            break;
        }
        leftover.extend_from_slice(&buf[..read as usize]);
        while let Some(pos) = leftover.iter().position(|&b| b == b'\n') {
            let mut line = leftover.drain(..=pos).collect::<Vec<_>>();
            if line.last() == Some(&b'\n') {
                line.pop();
            }
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            let text = String::from_utf8_lossy(&line);
            on_line(&text);
        }
    }
    if !leftover.is_empty() {
        let text = String::from_utf8_lossy(&leftover);
        if !text.is_empty() {
            on_line(text.trim_end_matches(['\r', '\n']));
        }
    }
}

pub fn quote_windows<'a>(args: impl IntoIterator<Item = &'a str>) -> String {
    args.into_iter()
        .map(quote_arg)
        .collect::<Vec<_>>()
        .join(" ")
}

fn quote_arg(arg: &str) -> String {
    if arg.is_empty() || arg.chars().any(|c| matches!(c, ' ' | '\t' | '"')) {
        let mut out = String::from("\"");
        let mut backslashes = 0;
        for c in arg.chars() {
            match c {
                '\\' => backslashes += 1,
                '"' => {
                    out.push_str(&"\\".repeat(backslashes * 2 + 1));
                    out.push('"');
                    backslashes = 0;
                }
                _ => {
                    out.push_str(&"\\".repeat(backslashes));
                    out.push(c);
                    backslashes = 0;
                }
            }
        }
        out.push_str(&"\\".repeat(backslashes * 2));
        out.push('"');
        out
    } else {
        arg.to_string()
    }
}

pub fn spawn_cmd_start_wait(title: &str, command: &str, args: &[String]) -> i32 {
    let mut spawn_args = vec![
        "/c".into(),
        "start".into(),
        "/wait".into(),
        title.into(),
        command.into(),
    ];
    spawn_args.extend(args.iter().cloned());
    let cmdline =
        quote_windows(std::iter::once("cmd.exe").chain(spawn_args.iter().map(String::as_str)));
    let mut cmd_wide = wide(&cmdline);
    unsafe {
        let mut si: StartupInfoW = std::mem::zeroed();
        si.cb = size_of::<StartupInfoW>() as Dword;
        let mut pi: ProcessInformation = std::mem::zeroed();
        if CreateProcessW(
            ptr::null(),
            cmd_wide.as_mut_ptr(),
            ptr::null_mut(),
            ptr::null_mut(),
            0,
            CREATE_NO_WINDOW,
            ptr::null_mut(),
            ptr::null(),
            &mut si,
            &mut pi,
        ) == 0
        {
            return 1;
        }
        WaitForSingleObject(pi.h_process, INFINITE);
        let mut code: Dword = 0;
        GetExitCodeProcess(pi.h_process, &mut code);
        CloseHandle(pi.h_process);
        CloseHandle(pi.h_thread);
        let _ = cmdline;
        code as i32
    }
}

#[allow(dead_code)]
const _: u32 = CP_UTF8;
#[allow(dead_code)]
const _: u32 = BCRYPT_HASH_LENGTH;
#[allow(dead_code)]
const _: u32 = BCRYPT_ALG_HANDLE_HMAC_FLAG;
