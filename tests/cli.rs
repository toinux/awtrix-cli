use std::{
    process::{Command, Output},
    thread,
    time::Duration,
};
use tiny_http::{Header, Response, Server};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_awtrix"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn help_version_and_offline_description_are_available() {
    assert!(run(&["--help"]).status.success());
    assert!(run(&["--version"]).status.success());
    let output = run(&["--json", "describe"]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("ESP32"));
}

#[test]
fn diagnose_reports_variant_version_and_capabilities_for_all_platforms() {
    for (variant, expected) in [
        ("esp32", "ESP32"),
        ("esp32-s3", "ESP32-S3"),
        ("tc002", "TC002"),
    ] {
        let server = Server::http("127.0.0.1:0").unwrap();
        let url = format!("http://{}", server.server_addr());
        let worker = thread::spawn(move || {
            for _ in 0..4 {
                let mut request = server.recv().unwrap();
                let mut body = String::new();
                request.as_reader().read_to_string(&mut body).unwrap();
                let (status, content) = match request.url() {
                    "/api/v1/system" => (200, format!(r#"{{"board":"{variant}"}}"#)),
                    "/api/v1/version" => (200, r#"{"version":"1.2.3"}"#.into()),
                    "/api/v1/device" => (200, r#"{"uptime":42}"#.into()),
                    "/api/v1/capabilities" => (200, r#"{"effects":["Fade"]}"#.into()),
                    _ => (404, "{}".into()),
                };
                let response = Response::from_string(content)
                    .with_status_code(status)
                    .with_header(Header::from_bytes("Content-Type", "application/json").unwrap());
                request.respond(response).unwrap();
            }
        });
        let output = run(&["--target", &url, "--json", "device", "diagnose"]);
        worker.join().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(json["variant"], expected);
        assert_eq!(json["version"], "1.2.3");
    }
}

#[test]
fn auth_failure_is_structured_and_does_not_leak_password() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        let r = server.recv().unwrap();
        assert!(r.headers().iter().any(|h| h.field.equiv("Authorization")));
        r.respond(Response::from_string("{}").with_status_code(401))
            .unwrap();
    });
    let output = run(&[
        "--target",
        &url,
        "--username",
        "user",
        "--password",
        "secret-value",
        "--json",
        "device",
        "state",
    ]);
    worker.join().unwrap();
    assert_eq!(output.status.code(), Some(3));
    let err = String::from_utf8_lossy(&output.stderr);
    assert!(err.contains("AUTHENTICATION"));
    assert!(!err.contains("secret-value"));
}

#[test]
fn field_selection_rejects_unknown_keys() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        let r = server.recv().unwrap();
        r.respond(Response::from_string(r#"{"uptime":1}"#)).unwrap();
    });
    let output = run(&["--target", &url, "--fields", "bogus", "device", "state"]);
    worker.join().unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("unknown field"));
}

#[test]
fn timeout_is_bounded_and_classified() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        let r = server.recv().unwrap();
        thread::sleep(Duration::from_millis(250));
        let _ = r.respond(Response::from_string("{}"));
    });
    let output = run(&[
        "--target",
        &url,
        "--timeout",
        "50",
        "--json",
        "device",
        "state",
    ]);
    worker.join().unwrap();
    assert_eq!(output.status.code(), Some(4));
    assert!(String::from_utf8_lossy(&output.stderr).contains("TIMEOUT"));
}

#[test]
fn invalid_json_response_is_classified() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        let r = server.recv().unwrap();
        r.respond(Response::from_string("not json")).unwrap();
    });
    let output = run(&["--target", &url, "--json", "device", "state"]);
    worker.join().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("INVALID_RESPONSE"));
}
