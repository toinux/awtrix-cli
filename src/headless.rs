//! Lifecycle for a caller-supplied AWTRIX Linux executable.
//!
//! Ownership contract: this CLI starts only a child it spawned. Stop operates on
//! the child PID recorded in the user's AWTRIX config and verifies Linux process
//! start-time before signaling; it never searches for or kills arbitrary processes.
use clap::{Args, Subcommand};
use serde_json::{json, Value};
#[cfg(target_os = "linux")]
use std::os::fd::{AsRawFd, FromRawFd};
use std::{
    path::PathBuf,
    process::{Child, Command as ProcessCommand, Stdio},
    thread,
    time::{Duration, Instant},
};

#[derive(Subcommand)]
pub(crate) enum CommandAction {
    Start(StartArgs),
    Stop,
    Status,
}
pub(crate) type Command = CommandAction;

#[derive(Args)]
pub(crate) struct StartArgs {
    #[arg(long, env = "AWTRIX_LINUX_BIN")]
    binary: Option<PathBuf>,
    #[arg(long, default_value_t = 8080)]
    port: u16,
    #[arg(long, default_value_t = 52)]
    width: u16,
    #[arg(long, default_value_t = 16)]
    height: u16,
    #[arg(long)]
    data: Option<PathBuf>,
    #[arg(long)]
    webui: Option<PathBuf>,
    #[arg(long, default_value_t = 15)]
    ready_timeout_secs: u64,
}

fn error(code: &'static str, message: impl Into<String>) -> (&'static str, String) {
    (code, message.into())
}

pub(crate) fn run(action: &Command) -> Result<Value, (&'static str, String)> {
    match action {
        Command::Start(args) => start(args),
        Command::Status => status(),
        Command::Stop => stop(),
    }
}

fn state_file() -> Result<PathBuf, (&'static str, String)> {
    let root = std::env::var_os("AWTRIX_CONFIG")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config/awtrix/config.json"))
        })
        .ok_or_else(|| error("CONFIG", "cannot determine AWTRIX config path"))?;
    Ok(root.with_file_name("headless.json"))
}

fn write_state(
    pid: u32,
    started: u64,
    url: &str,
    data: &std::path::Path,
    temporary: bool,
) -> Result<(), (&'static str, String)> {
    let path = state_file()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| error("FILE", e.to_string()))?;
    }
    let bytes = serde_json::to_vec(
        &json!({"pid":pid,"start_time":started,"url":url,"data":data,"temporary":temporary}),
    )
    .map_err(|e| error("FILE", e.to_string()))?;
    let parent = path
        .parent()
        .ok_or_else(|| error("FILE", "ownership record has no parent directory"))?;
    let mut file =
        tempfile::NamedTempFile::new_in(parent).map_err(|e| error("FILE", e.to_string()))?;
    use std::io::Write;
    file.write_all(&bytes)
        .map_err(|e| error("FILE", e.to_string()))?;
    file.persist(path)
        .map_err(|e| error("FILE", e.to_string()))?;
    Ok(())
}
fn read_state() -> Result<Value, (&'static str, String)> {
    let bytes = std::fs::read(state_file()?)
        .map_err(|_| error("NOT_RUNNING", "no CLI-owned headless instance is recorded"))?;
    serde_json::from_slice(&bytes)
        .map_err(|_| error("STATE_INVALID", "headless ownership record is invalid"))
}
#[cfg(target_os = "linux")]
fn process_start(pid: u32) -> Option<u64> {
    let data = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let after = data.rsplit_once(") ")?.1;
    after.split_whitespace().nth(19)?.parse().ok()
}
#[cfg(not(target_os = "linux"))]
fn process_start(_: u32) -> Option<u64> {
    None
}

/// Opens a stable kernel reference to a process; signaling through this fd cannot
/// accidentally target a different process if the numeric PID is recycled.
#[cfg(target_os = "linux")]
fn open_owned_process(
    pid: u32,
    expected_start: u64,
) -> Result<std::fs::File, (&'static str, String)> {
    let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid as libc::pid_t, 0) as i32 };
    if fd < 0 {
        return Err(error("NOT_RUNNING", "recorded process no longer exists"));
    }
    let process = unsafe { std::fs::File::from_raw_fd(fd) };
    if process_start(pid) != Some(expected_start) {
        return Err(error(
            "NOT_RUNNING",
            "recorded process identity no longer matches; no signal sent",
        ));
    }
    Ok(process)
}

fn start(args: &StartArgs) -> Result<Value, (&'static str, String)> {
    if !cfg!(target_os = "linux") {
        return Err(error(
            "UNSUPPORTED_HOST",
            "AWTRIX Linux headless can run only on Linux",
        ));
    }
    if !(1024..=65535).contains(&args.port)
        || !(8..=128).contains(&args.width)
        || !(8..=32).contains(&args.height)
        || args.ready_timeout_secs == 0
        || args.ready_timeout_secs > 300
    {
        return Err(error(
            "ARGUMENT",
            "port must be 1024..65535, width 8..128, height 8..32 and ready timeout 1..300 seconds",
        ));
    }
    let binary = args.binary.as_ref().ok_or_else(|| {
        error(
            "BINARY_REQUIRED",
            "provide --binary or AWTRIX_LINUX_BIN; automatic downloads are not supported",
        )
    })?;
    if !binary.is_file() {
        return Err(error(
            "BINARY_NOT_FOUND",
            format!("AWTRIX executable not found: {}", binary.display()),
        ));
    }
    if let Ok(existing) = read_state() {
        if existing["pid"]
            .as_u64()
            .and_then(|pid| process_start(pid as u32))
            == existing["start_time"].as_u64()
        {
            return Err(error(
                "ALREADY_RUNNING",
                "a CLI-owned headless process is already recorded",
            ));
        }
    }
    let (data, temporary) = match &args.data {
        Some(path) => (path.clone(), false),
        None => (
            std::env::temp_dir().join(format!(
                "awtrix-headless-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
            )),
            true,
        ),
    };
    std::fs::create_dir_all(&data).map_err(|e| error("DATA_DIR", e.to_string()))?;
    let mut process = ProcessCommand::new(binary);
    process.args([
        "--data",
        data.to_string_lossy().as_ref(),
        "--port",
        &args.port.to_string(),
        "--width",
        &args.width.to_string(),
        "--height",
        &args.height.to_string(),
    ]);
    if let Some(webui) = &args.webui {
        process.arg("--webui").arg(webui);
    }
    let child = match process
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            if temporary {
                let _ = std::fs::remove_dir_all(&data);
            }
            return Err(error(
                "SPAWN",
                format!("could not start AWTRIX executable: {e}"),
            ));
        }
    };
    await_ready(child, &data, temporary, args)
}

fn await_ready(
    mut child: Child,
    data: &std::path::Path,
    temporary: bool,
    args: &StartArgs,
) -> Result<Value, (&'static str, String)> {
    let pid = child.id();
    let deadline = Instant::now() + Duration::from_secs(args.ready_timeout_secs);
    let url = format!("http://127.0.0.1:{}", args.port);
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(250))
        .build()
    {
        Ok(client) => client,
        Err(e) => {
            let _ = child.kill();
            let _ = child.wait();
            if temporary {
                let _ = std::fs::remove_dir_all(data);
            }
            return Err(error("HTTP", e.to_string()));
        }
    };
    let interrupted = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let interrupt_flag = interrupted.clone();
    if let Err(e) =
        ctrlc::set_handler(move || interrupt_flag.store(true, std::sync::atomic::Ordering::SeqCst))
    {
        let _ = child.kill();
        let _ = child.wait();
        if temporary {
            let _ = std::fs::remove_dir_all(data);
        }
        return Err(error(
            "SIGNAL",
            format!("cannot install startup interrupt handler: {e}"),
        ));
    }
    loop {
        if interrupted.load(std::sync::atomic::Ordering::SeqCst) {
            let _ = child.kill();
            let _ = child.wait();
            if temporary {
                let _ = std::fs::remove_dir_all(data);
            }
            return Err(error(
                "INTERRUPTED",
                "headless startup interrupted; child stopped and temporary data removed",
            ));
        }
        if let Some(status) = child
            .try_wait()
            .map_err(|e| error("PROCESS", e.to_string()))?
        {
            if temporary {
                let _ = std::fs::remove_dir_all(data);
            }
            return Err(error(
                "START_FAILED",
                format!("AWTRIX exited before HTTP readiness ({status})"),
            ));
        }
        if client
            .get(format!("{url}/api/v1/device"))
            .send()
            .is_ok_and(|r| r.status().is_success())
        {
            let Some(started) = process_start(pid) else {
                let _ = child.kill();
                let _ = child.wait();
                if temporary {
                    let _ = std::fs::remove_dir_all(data);
                }
                return Err(error("OWNERSHIP", "cannot verify spawned process identity"));
            };
            if let Err(e) = write_state(pid, started, &url, data, temporary) {
                let _ = child.kill();
                let _ = child.wait();
                if temporary {
                    let _ = std::fs::remove_dir_all(data);
                }
                return Err(e);
            }
            return Ok(
                json!({"running":true,"pid":pid,"target":url,"data":data,"temporary_data":temporary,"owned":true}),
            );
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            if temporary {
                let _ = std::fs::remove_dir_all(data);
            }
            return Err(error("TIMEOUT",format!("AWTRIX HTTP readiness timed out after {} seconds; child stopped and temporary data removed",args.ready_timeout_secs)));
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn status() -> Result<Value, (&'static str, String)> {
    let s = read_state()?;
    let pid = s["pid"].as_u64().unwrap_or_default() as u32;
    let running = process_start(pid).is_some_and(|t| Some(t) == s["start_time"].as_u64());
    Ok(
        json!({"running":running,"owned":running,"pid":pid,"target":s["url"],"data":s["data"],"temporary_data":s["temporary"]}),
    )
}

fn stop() -> Result<Value, (&'static str, String)> {
    let s = read_state()?;
    let pid = s["pid"]
        .as_u64()
        .ok_or_else(|| error("STATE_INVALID", "missing PID"))? as u32;
    let expected = s["start_time"]
        .as_u64()
        .ok_or_else(|| error("STATE_INVALID", "missing process identity"))?;
    #[cfg(target_os = "linux")]
    {
        let process = open_owned_process(pid, expected)?;
        let result = unsafe {
            libc::syscall(
                libc::SYS_pidfd_send_signal,
                process.as_raw_fd(),
                libc::SIGTERM,
                std::ptr::null::<libc::siginfo_t>(),
                0,
            ) as i32
        };
        if result != 0 {
            return Err(error(
                "STOP_FAILED",
                std::io::Error::last_os_error().to_string(),
            ));
        }
    }
    #[cfg(not(target_os = "linux"))]
    return Err(error(
        "UNSUPPORTED_HOST",
        "AWTRIX process control requires Linux",
    ));
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline && process_start(pid) == Some(expected) {
        thread::sleep(Duration::from_millis(50));
    }
    if process_start(pid) == Some(expected) {
        return Err(error(
            "STOP_TIMEOUT",
            "AWTRIX did not stop after SIGTERM; refusing unsafe escalation",
        ));
    }
    if s["temporary"].as_bool() == Some(true) {
        if let Some(path) = s["data"].as_str() {
            std::fs::remove_dir_all(path).map_err(|e| error("CLEANUP", e.to_string()))?;
        }
    }
    let _ = std::fs::remove_file(state_file()?);
    Ok(json!({"stopped":true,"pid":pid,"data_removed":s["temporary"]}))
}

pub(crate) fn describe(topic: &str) -> Value {
    let action = topic.strip_prefix("headless ").unwrap_or("all");
    json!({"command":format!("headless {action}"),"parameters":{"--binary":"provided awtrix-linux executable; alternatively AWTRIX_LINUX_BIN","--port":"loopback HTTP port 1024..65535","--width":"display width 8..128","--height":"display height 8..32","--data":"persistent directory; omitted creates isolated temporary data","--webui":"optional AWTRIX web UI asset file path","--ready-timeout-secs":"bounded HTTP readiness wait, 1..300 (default 15)"},"outputs":["start returns owned target URL, PID and data directory","status reports only the recorded CLI-owned process","stop signals only a process matching the recorded Linux PID and start time"],"examples":["awtrix headless start --binary ./awtrix-linux","awtrix headless start --binary ./awtrix-linux --webui ./webui/index.html --data ./dev-data --port 8081 --width 52 --height 16","awtrix headless status","awtrix headless stop"],"prerequisites":["Linux host and caller-supplied AWTRIX Linux executable; no automatic download","headless does not validate sensors, audio, or real ESP32 memory/instruction budgets"]})
}
