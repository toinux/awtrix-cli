//! Incremental AWTRIX log reader. The device retains only a bounded ring buffer.
use clap::Subcommand;
use serde_json::{json, Value};
use std::{
    io::Write,
    sync::atomic::{AtomicBool, Ordering},
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

#[derive(Subcommand)]
pub enum Command {
    /// Read the current bounded device log buffer once.
    Read {
        #[arg(long, default_value_t = 0)]
        after: u64,
    },
    /// Follow new log lines until the bounded duration or Ctrl-C.
    Follow {
        #[arg(long, default_value_t = 0)]
        after: u64,
        #[arg(long, default_value_t = 1000, value_parser = clap::value_parser!(u64).range(1..=60000))]
        interval_ms: u64,
        #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(u64).range(1..=3600))]
        duration_secs: u64,
        /// Keep only lines containing this literal script-name prefix.
        #[arg(long)]
        script: Option<String>,
    },
}

pub fn run(command: &Command, api: &crate::ApiClient, machine: bool) -> crate::CliResult<Value> {
    match command {
        Command::Read { after } => {
            let (next, lines) = poll(api, *after)?;
            Ok(
                json!({"after":after,"next":next,"lines":lines,"history_limit":34,"exhaustive":false}),
            )
        }
        Command::Follow {
            after,
            interval_ms,
            duration_secs,
            script,
        } => follow(
            *after,
            *interval_ms,
            *duration_secs,
            script.as_deref(),
            machine,
            |cursor| poll(api, cursor),
        ),
    }
}

fn poll(api: &crate::ApiClient, cursor: u64) -> crate::CliResult<(u64, Vec<String>)> {
    let value = api.get(&format!("/api/v1/logs?after={cursor}"))?;
    let next = value.get("next").and_then(Value::as_u64).ok_or((
        "INVALID_RESPONSE",
        "device log response is missing numeric next cursor".into(),
    ))?;
    let lines = value
        .get("lines")
        .and_then(Value::as_array)
        .ok_or((
            "INVALID_RESPONSE",
            "device log response is missing lines".into(),
        ))?
        .iter()
        .map(|line| {
            line.as_str()
                .map(str::to_owned)
                .ok_or(("INVALID_RESPONSE", "device log line is not text".into()))
        })
        .collect::<crate::CliResult<Vec<_>>>()?;
    Ok((next.max(cursor), lines))
}

fn follow<F>(
    mut cursor: u64,
    interval_ms: u64,
    duration_secs: u64,
    script: Option<&str>,
    machine: bool,
    mut fetch: F,
) -> crate::CliResult<Value>
where
    F: FnMut(u64) -> crate::CliResult<(u64, Vec<String>)>,
{
    let stopped = Arc::new(AtomicBool::new(false));
    let signal = Arc::clone(&stopped);
    ctrlc::set_handler(move || signal.store(true, Ordering::SeqCst))
        .map_err(|_| ("INTERRUPT", "could not install interrupt handler".into()))?;
    let start = Instant::now();
    let deadline = Duration::from_secs(duration_secs);
    let mut output = std::io::BufWriter::new(std::io::stdout().lock());
    while start.elapsed() < deadline && !stopped.load(Ordering::SeqCst) {
        let (next, lines) = fetch(cursor)?;
        for line in lines {
            if script.is_none_or(|prefix| line.contains(prefix)) {
                let record = if machine {
                    json!({"type":"log","line":line,"cursor":next,"complete_history":false})
                        .to_string()
                } else {
                    line
                };
                writeln!(output, "{record}")
                    .map_err(|_| ("TRANSPORT", "could not write log output".into()))?;
                output
                    .flush()
                    .map_err(|_| ("TRANSPORT", "could not flush log output".into()))?;
            }
        }
        cursor = next;
        let remaining = deadline.saturating_sub(start.elapsed());
        if remaining.is_zero() {
            break;
        }
        let pause = Duration::from_millis(interval_ms).min(remaining);
        let pause_start = Instant::now();
        while pause_start.elapsed() < pause && !stopped.load(Ordering::SeqCst) {
            thread::sleep(
                Duration::from_millis(50).min(pause.saturating_sub(pause_start.elapsed())),
            );
        }
    }
    let end = json!({"type":"end","next":cursor,"interrupted":stopped.load(Ordering::SeqCst),"history_limit":34,"exhaustive":false});
    if machine {
        writeln!(output, "{end}")
            .map_err(|_| ("TRANSPORT", "could not write log output".into()))?;
    } else {
        writeln!(output, "-- follow ended at cursor {cursor}; device retains at most 34 lines, history may be incomplete --").map_err(|_| ("TRANSPORT", "could not write log output".into()))?;
    }
    output
        .flush()
        .map_err(|_| ("TRANSPORT", "could not flush log output".into()))?;
    Ok(json!({"follow_complete":true,"next":cursor,"interrupted":stopped.load(Ordering::SeqCst)}))
}

pub fn describe(topic: &str) -> crate::CliResult<Value> {
    Ok(
        json!({"command":topic,"parameters":{"--after":"resume after this sequence cursor (default 0)","--interval-ms":"poll interval, 1..60000 (default 1000)","--duration-secs":"follow limit, 1..3600 (default 30)","--script":"literal substring filter for a script prefix"},"inputs":["AWTRIX NG GET /api/v1/logs?after=<cursor>"],"outputs":{"read":["after","next","lines","history_limit","exhaustive"],"follow":["progressive human lines or JSONL {type:log,line,cursor,complete_history:false}; end {type:end,next,interrupted,history_limit,exhaustive:false}"]},"output_fields":["after","next","lines","history_limit","exhaustive","type","line","cursor","complete_history","interrupted"],"examples":["awtrix logs read --after 0","awtrix --json logs follow --after 12 --interval-ms 500 --duration-secs 60","awtrix logs follow --script weather"],"prerequisites":["AWTRIX NG HTTP endpoint; Basic credentials when enabled"],"limitations":"Device retains only its latest 34 lines (each at most 120 characters); older lines can be lost. No exhaustive history is promised."}),
    )
}
