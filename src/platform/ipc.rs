use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::fs::OpenOptions;

#[derive(Serialize, Deserialize, Debug)]
pub enum IpcMessage {
    OpenFiles {
        files: Vec<PathBuf>,
    },
    FocusWindow,
}

pub const PIPE_NAME: &str = r"\\.\pipe\neonote-ipc";

#[cfg(target_os = "windows")]
pub fn try_send_ipc_message(msg: &IpcMessage) -> anyhow::Result<()> {
    let mut pipe = OpenOptions::new()
        .write(true)
        .open(PIPE_NAME)?;
    
    let json = serde_json::to_string(msg)?;
    pipe.write_all(json.as_bytes())?;
    pipe.flush()?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn try_send_ipc_message(_msg: &IpcMessage) -> anyhow::Result<()> {
    anyhow::bail!("IPC not supported on this OS");
}

#[cfg(target_os = "windows")]
pub fn start_ipc_server<F>(on_message: F)
where
    F: Fn(IpcMessage) + Send + Sync + 'static,
{
    use windows::{
        core::PCWSTR,
        Win32::System::Pipes::{CreateNamedPipeW, ConnectNamedPipe, DisconnectNamedPipe, PIPE_TYPE_MESSAGE, PIPE_READMODE_MESSAGE, PIPE_WAIT},
        Win32::Foundation::{HANDLE, CloseHandle, GetLastError, ERROR_PIPE_CONNECTED},
        Win32::Storage::FileSystem::ReadFile,
        Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES,
    };
    use std::os::windows::ffi::OsStrExt;

    let callback = std::sync::Arc::new(on_message);

    std::thread::spawn(move || {
        let mut pipe_name: Vec<u16> = std::ffi::OsStr::new(PIPE_NAME).encode_wide().collect();
        pipe_name.push(0);

        loop {
            let handle = unsafe {
                CreateNamedPipeW(
                    PCWSTR(pipe_name.as_ptr()),
                    FILE_FLAGS_AND_ATTRIBUTES(3), // PIPE_ACCESS_DUPLEX
                    PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_WAIT,
                    1, // max instances
                    4096,
                    4096,
                    0, // default timeout
                    None,
                )
            };

            if handle.is_invalid() {
                // Could not create pipe (maybe another instance is already the server)
                break;
            }

            let connected = unsafe {
                let res = ConnectNamedPipe(handle, None);
                if res.is_ok() {
                    true
                } else {
                    GetLastError() == ERROR_PIPE_CONNECTED
                }
            };

            if connected {
                let mut buffer = vec![0u8; 4096];
                let mut bytes_read = 0;

                let success = unsafe {
                    ReadFile(
                        handle,
                        Some(buffer.as_mut_slice()),
                        Some(&mut bytes_read),
                        None,
                    )
                };

                if success.is_ok() && bytes_read > 0 {
                    let msg_str = String::from_utf8_lossy(&buffer[..bytes_read as usize]);
                    if let Ok(msg) = serde_json::from_str::<IpcMessage>(&msg_str) {
                        let cb = callback.clone();
                        cb(msg);
                    }
                }

                unsafe {
                    let _ = DisconnectNamedPipe(handle);
                }
            }
            
            unsafe {
                let _ = CloseHandle(handle);
            }
        }
    });
}

#[cfg(not(target_os = "windows"))]
pub fn start_ipc_server<F>(_on_message: F)
where
    F: Fn(IpcMessage) + Send + Sync + 'static,
{
}
