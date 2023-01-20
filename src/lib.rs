use detour::static_detour;
use lazy_static::lazy_static;
use std::{
    error::Error,
    ffi::{CString, NulError},
    io::{Read, Write},
    net::TcpStream,
    ops::Sub,
    sync::Mutex,
    time::{self, Duration, SystemTime, UNIX_EPOCH},
};
use winapi::{
    shared::minwindef::{FILETIME, LPFILETIME},
    um::libloaderapi::{GetModuleHandleA, GetProcAddress},
};
extern crate ctor;

lazy_static! {
    static ref TIME_CONFIG: Mutex<TimeConfig> = Mutex::new(TimeConfig::new(0, 0, true, true));
}
static_detour! {
    static get_system_time_as_file_time_detour: extern "system" fn(LPFILETIME);
}

/// Get the current windows FILETIME timestamp
fn get_real_time_stamp() -> u64 {
    (SystemTime::now()
        .duration_since(UNIX_EPOCH.sub(Duration::from_secs(11644473600)))
        .unwrap()
        .as_nanos()
        / 100) as u64
}

/// Configuration for spoofing time
struct TimeConfig {
    base_time: u64,
    new_time: u64,
    real_time: bool,
    move_forward: bool,
}

impl TimeConfig {
    fn new(base_time: u64, new_time: u64, real_time: bool, move_forward: bool) -> TimeConfig {
        TimeConfig {
            base_time,
            new_time,
            real_time,
            move_forward,
        }
    }

    /// Get the current spoofed windows FILETIME timestamp
    fn get_current_timestamp(&self) -> u64 {
        if !(self.real_time || self.move_forward) {
            return self.new_time;
        }

        let real_time_stamp = get_real_time_stamp();

        if self.real_time {
            return real_time_stamp;
        }

        if self.move_forward {
            return self
                .new_time
                .wrapping_sub(self.base_time)
                .wrapping_add(real_time_stamp);
        }

        0
    }

    /// Get the current spoofed windows FILETIME timestamp as a FILETIME object
    fn get_current_filetime(&self) -> FILETIME {
        let ts = self.get_current_timestamp();
        FILETIME {
            dwLowDateTime: (ts & 0xFFFFFFFF) as u32,
            dwHighDateTime: (ts >> 32) as u32,
        }
    }

    /// Update config values and base time
    fn update_settings(
        &mut self,
        timestamp: u64,
        real_time: bool,
        move_forward: bool,
        update_base_time: bool,
    ) {
        if update_base_time {
            self.base_time = get_real_time_stamp();
        }
        self.new_time = timestamp;
        self.real_time = real_time;
        self.move_forward = move_forward;
    }
}

#[derive(Debug)]
/// Generic error for anything that can go wrong while hooking the process
struct HookError {
    message: String,
}

impl HookError {
    fn new(msg: &str) -> HookError {
        HookError {
            message: msg.to_string(),
        }
    }
}

impl std::fmt::Display for HookError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for HookError {}
impl From<detour::Error> for HookError {
    fn from(value: detour::Error) -> Self {
        HookError {
            message: format!("detour::Error {:?}", value),
        }
    }
}
impl From<NulError> for HookError {
    fn from(value: NulError) -> Self {
        HookError {
            message: format!("NulError {:?}", value),
        }
    }
}
impl From<std::io::Error> for HookError {
    fn from(value: std::io::Error) -> Self {
        HookError {
            message: format!("std::io::Error {:?}", value),
        }
    }
}

/// Send a string through TcpStream
fn log(stream: &TcpStream, msg: &str) -> Result<(), std::io::Error> {
    let mut stream_clone = stream.try_clone()?;
    stream_clone.write(msg.as_bytes())?;

    Ok(())
}

/// Recieve a buffer from TcpStream
fn recv(stream: &TcpStream) -> Result<([u8; 1024], usize), std::io::Error> {
    let mut stream_clone = stream.try_clone()?;
    let mut buf = [0u8; 1024];
    let size = stream_clone.read(&mut buf)?;

    Ok((buf, size))
}

/// Find and hook GetSystemTimeAsFileTime in the target application
unsafe fn find_hook_fn() -> Result<(), HookError> {
    // assume server is running and able to be connected to
    let stream = TcpStream::connect("127.0.0.1:63463")?;
    // read timeout in order to prevent halting the process calling GetSystemTimeAsFileTime
    stream.set_read_timeout(Some(time::Duration::new(0, 100)))?;

    // find GetSystemTimeAsFileTime
    let module_name = CString::new("kernel32.dll")?;
    let function_name = CString::new("GetSystemTimeAsFileTime")?;
    let kernel32 = GetModuleHandleA(module_name.as_ptr());
    if kernel32.is_null() {
        return Err(HookError::new("Failed to find kernel32.dll"));
    }
    let get_system_time_as_file_time = GetProcAddress(kernel32, function_name.as_ptr());
    if get_system_time_as_file_time.is_null() {
        println!("Failed to get function handle!");
        return Err(HookError::new("Failed to find GetSystemTimeAsFileTime"));
    }
    let get_system_time_as_file_time = std::mem::transmute(get_system_time_as_file_time);

    // hook into GetSystemTimeAsFileTime to call our spoofed function
    let hook = get_system_time_as_file_time_detour
        .initialize(get_system_time_as_file_time, move |file_time_ptr| {
            get_system_time_as_file_time_hook(file_time_ptr, &stream)
        })?;
    hook.enable()?;

    Ok(())
}

/// To be run when the DLL is loaded
#[ctor::ctor]
fn ctor() {
    unsafe { find_hook_fn().unwrap() };
}

/// To be run when GetSystemTimeAsFileTime is called, check for updates from TcpStream and inject spoofed FILETIME
extern "system" fn get_system_time_as_file_time_hook(
    file_time_ptr: LPFILETIME,
    stream: &TcpStream,
) {
    let mut time_config = TIME_CONFIG.lock().unwrap();
    // check with the server for any new settings
    match recv(stream) {
        Err(_) => (),
        Ok((buf, size)) => {
            // assume that the server plays nice and sends properly formatted data
            let buf = &buf[0..size];
            let command: Vec<&str> = std::str::from_utf8(&buf).unwrap().split(" ").collect();
            let timestamp = str::parse::<u64>(command[0]).unwrap();
            let real_time = str::parse::<i8>(command[1]).unwrap() == 1;
            let move_forward = str::parse::<i8>(command[2]).unwrap() == 1;
            let update_base_time = str::parse::<i8>(command[3]).unwrap() == 1;
            time_config.update_settings(timestamp, real_time, move_forward, update_base_time);
            // echo parsed data
            log(
                stream,
                format!(
                    "{:?} {} {} {} {}",
                    command, timestamp, real_time, move_forward, update_base_time
                )
                .as_str(),
            )
            // errors here should be ignored to avoid crashing the process in the event that the server is no longer active
            .unwrap_or(());
        }
    };
    // inject spoofed time
    let filetime = time_config.get_current_filetime();
    unsafe {
        file_time_ptr.write(filetime);
    }
}
