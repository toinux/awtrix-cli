use std::{process::Command, thread};
use tiny_http::{Response, Server};

#[test]
fn packaged_binary_help_and_version_work_without_rust_environment() {
    for args in [&["--help"][..], &["--version"][..]] {
        let output = Command::new(env!("CARGO_BIN_EXE_awtrix"))
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
    let output = Command::new(env!("CARGO_BIN_EXE_awtrix"))
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
