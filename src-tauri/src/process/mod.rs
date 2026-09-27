use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use portable_pty::{CommandBuilder, MasterPty, NativePtySystem, PtySize, PtySystem};

use crate::error::{Error, Result};

/// Setup and run commands in a PTY, each in its own process group.
#[derive(Default)]
pub struct ProcessSupervisor {
    current: Mutex<Option<RunningApp>>,
}

struct RunningApp {
    pid: u32,
    output: Arc<Mutex<String>>,
    exited: Arc<Mutex<Option<i32>>>,
    killed: Arc<AtomicBool>,
    _master: Box<dyn MasterPty + Send>,
}

impl ProcessSupervisor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn spawn(&self, program: &str, args: &[&str]) -> Result<u32> {
        let pty_system = NativePtySystem::default();
        let pair = pty_system
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|error| Error::Process(error.to_string()))?;

        let mut cmd = CommandBuilder::new(program);
        for arg in args {
            cmd.arg(*arg);
        }

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|error| Error::Process(error.to_string()))?;
        let pid = child
            .process_id()
            .ok_or_else(|| Error::Process("pty child has no pid".into()))?;

        drop(pair.slave);

        let output = Arc::new(Mutex::new(String::new()));
        let exited = Arc::new(Mutex::new(None));
        let killed = Arc::new(AtomicBool::new(false));
        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|error| Error::Process(error.to_string()))?;
        let output_reader = output.clone();
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 {
                    break;
                }
                if let Ok(mut text) = output_reader.lock() {
                    text.push_str(&String::from_utf8_lossy(&buf[..n]));
                }
            }
        });

        let wait_exited = exited.clone();
        thread::spawn(move || {
            let mut child = child;
            let code = match child.wait() {
                Ok(status) => i32::try_from(status.exit_code()).unwrap_or(1),
                Err(_) => 1,
            };
            if let Ok(mut slot) = wait_exited.lock() {
                *slot = Some(code);
            }
        });

        *self.current.lock().unwrap() = Some(RunningApp {
            pid,
            output,
            exited,
            killed,
            _master: pair.master,
        });
        Ok(pid)
    }

    pub fn output(&self) -> String {
        self.current
            .lock()
            .unwrap()
            .as_ref()
            .and_then(|app| app.output.lock().ok().map(|text| text.clone()))
            .unwrap_or_default()
    }

    pub fn detected_port(&self) -> Option<u16> {
        detect_port(&self.output())
    }

    pub fn current_pid(&self) -> Option<u32> {
        self.current.lock().unwrap().as_ref().map(|app| app.pid)
    }

    pub fn stop(&self) -> Result<()> {
        let guard = self.current.lock().unwrap();
        let Some(app) = guard.as_ref() else {
            return Ok(());
        };
        app.killed.store(true, Ordering::SeqCst);
        kill_group(app.pid)
    }

    pub fn wait_exit(&self, timeout: Duration) -> Result<i32> {
        let started = std::time::Instant::now();
        loop {
            if let Some(code) = self
                .current
                .lock()
                .unwrap()
                .as_ref()
                .and_then(|app| app.exited.lock().ok().and_then(|slot| *slot))
            {
                return Ok(code);
            }
            if started.elapsed() > timeout {
                return Err(Error::Process("timed out waiting for exit".into()));
            }
            thread::sleep(Duration::from_millis(50));
        }
    }

    pub async fn stop_all(&self) -> Result<()> {
        self.stop()
    }
}

pub fn detect_port(output: &str) -> Option<u16> {
    for marker in ["localhost:", "127.0.0.1:"] {
        if let Some(index) = output.find(marker) {
            let digits: String = output[index + marker.len()..]
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            if let Ok(port) = digits.parse() {
                return Some(port);
            }
        }
    }
    None
}

fn kill_group(pid: u32) -> Result<()> {
    let pgid = pid as i32;
    let result = unsafe { libc::killpg(pgid, libc::SIGTERM) };
    if result != 0 {
        unsafe {
            libc::kill(pgid, libc::SIGTERM);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn python_script(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src/process")
            .join(name)
    }

    #[test]
    fn reads_port_from_dev_server_banner() {
        assert_eq!(detect_port("Local:   http://127.0.0.1:5173/\n"), Some(5173));
        assert_eq!(detect_port("ready on localhost:3000"), Some(3000));
    }

    #[test]
    fn pty_detects_port_stop_kills_tree_crash_reports_code() {
        let supervisor = ProcessSupervisor::new();
        let script = python_script("fake_dev_server.py");
        supervisor
            .spawn("python3", &[script.to_str().unwrap()])
            .unwrap();

        let started = std::time::Instant::now();
        let port = loop {
            if let Some(port) = supervisor.detected_port() {
                break port;
            }
            if started.elapsed() > Duration::from_secs(5) {
                panic!("no port in output: {}", supervisor.output());
            }
            thread::sleep(Duration::from_millis(50));
        };
        assert_eq!(port, 8765);

        let pid = supervisor.current_pid().unwrap();
        supervisor.stop().unwrap();
        thread::sleep(Duration::from_millis(400));
        let still_alive = unsafe { libc::kill(pid as i32, 0) } == 0;
        assert!(!still_alive, "process group should be dead after Stop");

        let supervisor = ProcessSupervisor::new();
        let crash = python_script("fake_crash.py");
        supervisor
            .spawn("python3", &[crash.to_str().unwrap()])
            .unwrap();
        let code = supervisor.wait_exit(Duration::from_secs(5)).unwrap();
        assert_eq!(code, 17);
    }
}
