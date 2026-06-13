pub mod fallback;
pub mod hwnd;

use std::{
    env,
    net::TcpListener,
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

use egui::Rect;

use self::{fallback::FallbackEmbedder, hwnd::WindowHandle};

#[derive(Debug)]
pub enum EmbedError {
    NeovideNotFound,
    SpawnFailed(String),
    NoFreePort(String),
    HwndTimeout,
}

pub struct NeovideInstance {
    pub process_id: u32,
    pub rpc_port: u16,
    pub hwnd: Option<WindowHandle>,
    child: Child,
    fallback: FallbackEmbedder,
}

impl NeovideInstance {
    pub fn spawn() -> Result<Self, EmbedError> {
        let path = resolve_neovide_path().ok_or(EmbedError::NeovideNotFound)?;
        let rpc_port = reserve_local_port()?;
        let mut command = Command::new(path);
        command
            .arg("--no-fork")
            .arg("--neovim-bin")
            .arg("nvim")
            .arg("--listen")
            .arg(format!("127.0.0.1:{rpc_port}"))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000);
        }

        let child = command
            .spawn()
            .map_err(|error| EmbedError::SpawnFailed(error.to_string()))?;

        Ok(Self {
            process_id: child.id(),
            rpc_port,
            hwnd: None,
            child,
            fallback: FallbackEmbedder::default(),
        })
    }

    pub fn discover_hwnd(&mut self, timeout: Duration) -> Result<WindowHandle, EmbedError> {
        let start = Instant::now();
        while start.elapsed() < timeout {
            if let Some(hwnd) = hwnd::find_window_for_pid(self.process_id) {
                self.hwnd = Some(hwnd);
                return Ok(hwnd);
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        Err(EmbedError::HwndTimeout)
    }

    pub fn resize(&self, rect: Rect) {
        if let Some(hwnd) = self.hwnd {
            hwnd::move_window(
                hwnd,
                rect.width().round() as i32,
                rect.height().round() as i32,
            );
        }
    }

    pub fn has_exited(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(Some(_)))
    }
}

#[derive(Default)]
pub struct NeovideManager {
    active_rect: Option<Rect>,
}

impl NeovideManager {
    pub fn sync_editor_rect(&mut self, rect: Rect) {
        self.active_rect = Some(rect);
    }

    pub fn active_rect(&self) -> Option<Rect> {
        self.active_rect
    }
}

fn reserve_local_port() -> Result<u16, EmbedError> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|error| EmbedError::NoFreePort(error.to_string()))?;
    let port = listener
        .local_addr()
        .map_err(|error| EmbedError::NoFreePort(error.to_string()))?
        .port();
    drop(listener);
    Ok(port)
}

pub fn resolve_neovide_path() -> Option<PathBuf> {
    if let Ok(current_exe) = env::current_exe() {
        if let Some(dir) = current_exe.parent() {
            let bundled = dir.join("neovide.exe");
            if bundled.exists() {
                return Some(bundled);
            }
        }
    }

    if let Ok(path) = env::var("NEOVIDE_PATH") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Some(path);
        }
    }

    find_on_path("neovide.exe").or_else(|| find_on_path("neovide"))
}

fn find_on_path(binary: &str) -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;
    env::split_paths(&path_var)
        .map(|dir| dir.join(binary))
        .find(|candidate| candidate.exists())
}
