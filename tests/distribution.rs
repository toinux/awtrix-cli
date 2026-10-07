use std::{process::Command, thread};
use tiny_http::{Response, Server};

fn packaged_binary() -> Option<std::path::PathBuf> {
    let path = std::env::var_os("AWTRIX_DISTRIBUTION_BINARY")?;
    let path = std::path::PathBuf::from(path);
    assert!(
        path.is_file(),
        "release artifact does not exist: {}",
        path.display()
    );
    Some(path)
}

#[test]
fn packaged_binary_help_and_version_work_without_rust_environment() {
    let Some(binary) = packaged_binary() else {
        eprintln!("skipping packaged-artifact check outside the distribution workflow");
        return;
    };
    for args in [&["--help"][..], &["--version"][..]] {
        let output = Command::new(&binary)
            .args(args)
            .env_remove("RUSTUP_HOME")
            .env_remove("CARGO_HOME")
            .output()
            .expect("run standalone CLI");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!output.stdout.is_empty());
    }
}

#[test]
fn packaged_binary_diagnoses_http_target() {
    let Some(binary) = packaged_binary() else {
        eprintln!("skipping packaged-artifact check outside the distribution workflow");
        return;
    };
    let server = Server::http("127.0.0.1:0").unwrap();
    let target = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        for _ in 0..3 {
            let request = server.recv().unwrap();
            let body = match request.url() {
                "/api/v1/device" => r#"{"boardType":"awtrixng","soc":"esp32"}"#,
                "/api/v1/version" => r#"{"version":"test"}"#,
                _ => r#"{"effects":[]}"#,
            };
            request.respond(Response::from_string(body)).unwrap();
        }
    });
    let output = Command::new(binary)
        .args(["--target", &target, "--json", "device", "diagnose"])
        .env_remove("RUSTUP_HOME")
        .env_remove("CARGO_HOME")
        .output()
        .expect("run standalone CLI");
    worker.join().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["variant"], "ESP32");
}
