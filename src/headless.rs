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
    /// Start a caller-supplied local Linux executable and record ownership after HTTP readiness.
    Start(StartArgs),
    /// SIGTERM only the recorded owned process; wait up to five seconds and clean temporary data.
    Stop,
    /// Check recorded process existence and identity; does not probe HTTP health.
    Status,
}
pub(crate) type Command = CommandAction;

#[derive(Args)]
pub(crate) struct StartArgs {
    /// Caller-supplied AWTRIX Linux executable; no automatic download.
    #[arg(long, env = "AWTRIX_LINUX_BIN")]
    binary: Option<PathBuf>,
    /// Loopback HTTP port (1024..65535).
    #[arg(long, default_value_t = 8080)]
    port: u16,
    /// Framebuffer width (8..128).
    #[arg(long, default_value_t = 52)]
    width: u16,
    /// Framebuffer height (8..32).
    #[arg(long, default_value_t = 16)]
    height: u16,
    /// Persistent data directory retained after stop/errors; omitted creates temporary data.
    #[arg(long)]
    data: Option<PathBuf>,
    /// Optional caller-supplied Web UI asset file.
    #[arg(long)]
    webui: Option<PathBuf>,
    /// Startup HTTP readiness bound (1..300 seconds).
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
    crate::profiles::path()
        .map(|config| config.with_file_name("headless.json"))
        .map_err(|(_, message)| error("CONFIG", message))
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

#[cfg(target_os = "linux")]
fn owns_http_listener(pid: u32, port: u16) -> bool {
    let Ok(entries) = std::fs::read_dir(format!("/proc/{pid}/fd")) else {
        return false;
    };
    let sockets: std::collections::HashSet<String> = entries
        .flatten()
        .filter_map(|entry| {
            let target = std::fs::read_link(entry.path())
                .ok()?
                .to_string_lossy()
                .into_owned();
            target
                .strip_prefix("socket:[")
                .and_then(|s| s.strip_suffix(']'))
                .map(str::to_owned)
        })
        .collect();
    ["/proc/net/tcp", "/proc/net/tcp6"]
        .into_iter()
        .any(|table| {
            let Ok(content) = std::fs::read_to_string(table) else {
                return false;
            };
            content.lines().skip(1).any(|line| {
                let fields: Vec<_> = line.split_whitespace().collect();
                fields.len() > 9
                    && fields[3] == "0A"
                    && fields[1]
                        .rsplit_once(':')
                        .and_then(|(_, p)| u16::from_str_radix(p, 16).ok())
                        == Some(port)
                    && sockets.contains(fields[9])
            })
        })
}

#[cfg(not(target_os = "linux"))]
fn owns_http_listener(_: u32, _: u16) -> bool {
    false
}

struct StartupGuard {
    child: Option<Child>,
    data: PathBuf,
    temporary: bool,
}

impl StartupGuard {
    fn new(child: Child, data: &std::path::Path, temporary: bool) -> Self {
        Self {
            child: Some(child),
            data: data.to_path_buf(),
            temporary,
        }
    }
    fn child_mut(&mut self) -> &mut Child {
        self.child.as_mut().expect("startup guard owns child")
    }
    fn disarm(&mut self) {
        self.child = None;
        self.temporary = false;
    }
}

impl Drop for StartupGuard {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
        if self.temporary {
            let _ = std::fs::remove_dir_all(&self.data);
        }
    }
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
    let (value, _child) = launch(args, true, None)?;
    Ok(value)
}

/// A transient test-owned AWTRIX process. Unlike the interactive lifecycle, it never
/// reads or writes the user's ownership record and always kills only its child on drop.
pub(crate) struct IsolatedSession {
    child: Option<Child>,
    pub(crate) target: String,
    pub(crate) data: PathBuf,
}

impl Drop for IsolatedSession {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

pub(crate) fn start_isolated(
    binary: PathBuf,
    webui: Option<PathBuf>,
    data: PathBuf,
    port: u16,
    ready_timeout_secs: u64,
    interrupted: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Result<IsolatedSession, (&'static str, String)> {
    let data_dir = data.clone();
    let args = StartArgs {
        binary: Some(binary),
        port,
        width: 52,
        height: 16,
        data: Some(data),
        webui,
        ready_timeout_secs,
    };
    let (value, child) = launch(&args, false, Some(interrupted))?;
    Ok(IsolatedSession {
        child: Some(child),
        target: value["target"].as_str().unwrap_or_default().to_owned(),
        data: data_dir,
    })
}

fn launch(
    args: &StartArgs,
    persist_ownership: bool,
    cancellation: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
) -> Result<(Value, Child), (&'static str, String)> {
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
    if persist_ownership {
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
    }
    let reservation = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, args.port))
        .map_err(|_| {
            error(
                "PORT_IN_USE",
                format!("loopback port {} is already in use", args.port),
            )
        })?;
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
    if let Err(e) = std::fs::create_dir_all(&data) {
        if temporary {
            let _ = std::fs::remove_dir_all(&data);
        }
        return Err(error("DATA_DIR", e.to_string()));
    }
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
    // The reservation detects an already-occupied port before any child is spawned.
    // Release immediately before spawn; later ownership verification closes this race.
    drop(reservation);
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
    await_ready(
        child,
        &data,
        temporary,
        args,
        persist_ownership,
        cancellation,
    )
}

fn await_ready(
    child: Child,
    data: &std::path::Path,
    temporary: bool,
    args: &StartArgs,
    persist_ownership: bool,
    cancellation: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
) -> Result<(Value, Child), (&'static str, String)> {
    let mut startup = StartupGuard::new(child, data, temporary);
    let pid = startup.child_mut().id();
    let deadline = Instant::now() + Duration::from_secs(args.ready_timeout_secs);
    let url = format!("http://127.0.0.1:{}", args.port);
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(250))
        .build()
    {
        Ok(client) => client,
        Err(e) => {
            return Err(error("HTTP", e.to_string()));
        }
    };
    let interrupted = if let Some(flag) = cancellation {
        flag
    } else {
        let flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let interrupt_flag = flag.clone();
        if let Err(e) = ctrlc::set_handler(move || {
            interrupt_flag.store(true, std::sync::atomic::Ordering::SeqCst)
        }) {
            return Err(error(
                "SIGNAL",
                format!("cannot install startup interrupt handler: {e}"),
            ));
        }
        flag
    };
    loop {
        if interrupted.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(error(
                "INTERRUPTED",
                "headless startup interrupted; child stopped and temporary data removed",
            ));
        }
        let status = match startup.child_mut().try_wait() {
            Ok(status) => status,
            Err(e) => {
                return Err(error(
                    "PROCESS",
                    format!("failed checking child process; cleanup attempted: {e}"),
                ))
            }
        };
        if let Some(status) = status {
            return Err(error(
                "START_FAILED",
                format!("AWTRIX exited before HTTP readiness ({status})"),
            ));
        }
        if client
            .get(format!("{url}/api/v1/device"))
            .send()
            .is_ok_and(|r| r.status().is_success())
            && owns_http_listener(pid, args.port)
        {
            if let Some(status) = startup.child_mut().try_wait().map_err(|e| {
                error(
                    "PROCESS",
                    format!("failed checking child after HTTP readiness: {e}"),
                )
            })? {
                return Err(error(
                    "START_FAILED",
                    format!("AWTRIX exited during HTTP readiness ({status})"),
                ));
            }
            let Some(started) = process_start(pid) else {
                return Err(error("OWNERSHIP", "cannot verify spawned process identity"));
            };
            if persist_ownership {
                write_state(pid, started, &url, data, temporary)?;
            }
            let child = startup
                .child
                .take()
                .ok_or_else(|| error("PROCESS", "owned process disappeared during startup"))?;
            startup.disarm();
            return Ok((
                json!({"running":true,"pid":pid,"target":url,"data":data,"temporary_data":temporary,"owned":true}),
                child,
            ));
        }
        if Instant::now() >= deadline {
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
    #[cfg(not(target_os = "linux"))]
    return Err(error(
        "UNSUPPORTED_HOST",
        "AWTRIX process control requires Linux",
    ));

    #[cfg(target_os = "linux")]
    {
        let s = read_state()?;
        let pid = s["pid"]
            .as_u64()
            .ok_or_else(|| error("STATE_INVALID", "missing PID"))? as u32;
        let expected = s["start_time"]
            .as_u64()
            .ok_or_else(|| error("STATE_INVALID", "missing process identity"))?;
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
}

pub(crate) fn describe(topic: &str) -> Value {
    let start_parameters = json!({"--binary":"provided awtrix-linux executable; alternatively AWTRIX_LINUX_BIN; no automatic download","--port":"loopback HTTP port 1024..65535 (default 8080)","--width":"display width 8..128 (default 52)","--height":"display height 8..32 (default 16)","--data":"persistent directory retained on stop or startup failure; omitted creates temporary data removed on successful stop or startup failure/Ctrl-C","--webui":"optional caller-supplied Web UI asset file","--ready-timeout-secs":"bounded HTTP readiness wait 1..300 seconds (default 15)"});
    let (purpose, parameters, output_fields, examples) = match topic {
        "headless start" => (
            "Launch a local Linux child, verify its own HTTP listener, and record ownership",
            start_parameters.clone(),
            json!(["running", "pid", "target", "data", "temporary_data", "owned"]),
            json!(["awtrix-cli headless start --binary ./awtrix-linux", "awtrix-cli headless start --binary ./awtrix-linux --webui ./webui/index.html --data ./dev-data --port 8081 --width 52 --height 16"]),
        ),
        "headless status" => (
            "Check existence and start-time identity of the locally recorded process; no HTTP health probe",
            json!({}),
            json!(["running", "owned", "pid", "target", "data", "temporary_data"]),
            json!(["awtrix-cli --json headless status"]),
        ),
        "headless stop" => (
            "Signal only the recorded owned process, wait up to five seconds, and clean temporary data",
            json!({}),
            json!(["stopped", "pid", "data_removed"]),
            json!(["awtrix-cli --json headless stop"]),
        ),
        _ => (
            "Manage the local lifecycle of a caller-supplied AWTRIX Linux executable",
            json!({"start":start_parameters,"status":"no operation-specific options","stop":"no operation-specific options"}),
            json!({"start":["running","pid","target","data","temporary_data","owned"],"status":["running","owned","pid","target","data","temporary_data"],"stop":["stopped","pid","data_removed"]}),
            json!(["awtrix-cli headless start --binary ./awtrix-linux", "awtrix-cli headless status", "awtrix-cli headless stop"]),
        ),
    };
    let process_outputs = json!({"running":"recorded process exists with matching start time; status does not establish HTTP health","owned":"CLI process ownership confirmed","pid":"local child PID","target":"loopback endpoint, not selected by --target","data":"local data directory","temporary_data":"whether data is disposable and removed on successful stop"});
    let stop_outputs = json!({"stopped":"true after owned process exits","pid":"recorded local PID","data_removed":"true for temporary data, false for retained explicit --data"});
    let outputs = match topic {
        "headless start" | "headless status" => process_outputs,
        "headless stop" => stop_outputs,
        _ => json!({"start":process_outputs,"status":process_outputs,"stop":stop_outputs}),
    };
    json!({"command":topic,"purpose":purpose,"parameters":parameters,"global_options":{"--json":"compact JSON report","--fields":"comma-separated top-level report fields"},"output_fields":output_fields,"outputs":outputs,"ownership_record":"headless.json beside the selected personal config; AWTRIX_CONFIG selects the config file, otherwise $HOME/.config/awtrix-cli/config.json on Linux","lifecycle":{"target_selection":"--target, AWTRIX_URL and profile/default device targets are irrelevant for local start/status/stop","readiness":"successful /api/v1/device response plus spawned PID ownership of the listening socket","stop":"SIGTERM via pidfd after start-time identity check; waits five seconds; STOP_TIMEOUT does not escalate","cleanup":"startup errors/Ctrl-C kill and reap the spawned child and remove temporary data; successful stop removes temporary data and ownership record; explicit --data directories are retained","status":"reads ownership record and /proc only; missing record returns NOT_RUNNING"},"examples":examples,"prerequisites":["Linux host; start requires a caller-supplied AWTRIX Linux executable","headless does not validate sensors, audio, physical LEDs, or ESP32 memory/instruction budgets"]})
}
