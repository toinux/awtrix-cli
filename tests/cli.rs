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

fn run_with_url_env(args: &[&str], url: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_awtrix"))
        .args(args)
        .env("AWTRIX_URL", url)
        .output()
        .unwrap()
}

fn run_with_config(args: &[&str], config: &std::path::Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_awtrix"))
        .args(args)
        .env("AWTRIX_CONFIG", config)
        .env_remove("AWTRIX_URL")
        .env_remove("AWTRIX_PROFILE")
        .env_remove("AWTRIX_USERNAME")
        .env_remove("AWTRIX_PASSWORD")
        .output()
        .unwrap()
}

#[test]
fn profile_crud_persists_isolated_config_and_never_displays_secrets() {
    let dir = std::env::temp_dir().join(format!("awtrix-profile-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let config = dir.join("config.json");
    let added = run_with_config(
        &[
            "--json",
            "profile",
            "add",
            "desk",
            "--target",
            "http://127.0.0.1:1234",
            "--username",
            "private-user",
            "--password",
            "private-password",
        ],
        &config,
    );
    assert!(added.status.success());
    assert!(run_with_config(
        &[
            "--json",
            "profile",
            "update",
            "desk",
            "--target",
            "http://127.0.0.1:1235",
            "--username",
            "private-user",
            "--password",
            "private-password"
        ],
        &config
    )
    .status
    .success());
    assert!(
        run_with_config(&["--json", "profile", "set-default", "desk"], &config)
            .status
            .success()
    );
    for args in [
        &["--json", "profile", "list"][..],
        &["--json", "profile", "show", "desk"][..],
    ] {
        let output = run_with_config(args, &config);
        assert!(output.status.success());
        let text = String::from_utf8_lossy(&output.stdout);
        assert!(!text.contains("private-user"));
        assert!(!text.contains("private-password"));
    }
    let other = dir.join("other.json");
    assert!(serde_json::from_slice::<serde_json::Value>(
        &run_with_config(&["--json", "profile", "list"], &other).stdout
    )
    .unwrap()["profiles"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(
        run_with_config(&["--json", "profile", "delete", "desk"], &config)
            .status
            .success()
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn invalid_saved_profile_configuration_fails_explicitly() {
    let path = std::env::temp_dir().join(format!("awtrix-invalid-{}.json", std::process::id()));
    std::fs::write(
        &path,
        br#"{"profiles":{"broken":{"target":"file:///etc/passwd"}},"default":"broken"}"#,
    )
    .unwrap();
    let out = run_with_config(&["--json", "profile", "list"], &path);
    let _ = std::fs::remove_file(path);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&out.stdout).unwrap()["error"]["code"],
        "CONFIG_INVALID"
    );
}

#[test]
fn selected_profile_credentials_are_sent_and_explicit_credentials_override_them() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        for expected in [
            "Basic cHJvZmlsZS11c2VyOnByb2ZpbGUtcGFzc3dvcmQ=",
            "Basic Y2xpLXVzZXI6Y2xpLXBhc3N3b3Jk",
        ] {
            let request = server.recv().unwrap();
            let auth = request
                .headers()
                .iter()
                .find(|h| h.field.equiv("Authorization"))
                .unwrap()
                .value
                .as_str();
            assert_eq!(auth, expected);
            request.respond(Response::from_string("{}")).unwrap();
        }
    });
    let path = std::env::temp_dir().join(format!("awtrix-auth-{}.json", std::process::id()));
    let output = Command::new(env!("CARGO_BIN_EXE_awtrix"))
        .args([
            "profile",
            "add",
            "auth",
            "--target",
            &url,
            "--username",
            "profile-user",
            "--password",
            "profile-password",
        ])
        .env("AWTRIX_CONFIG", &path)
        .env_remove("AWTRIX_URL")
        .output()
        .unwrap();
    assert!(output.status.success());
    let _ = Command::new(env!("CARGO_BIN_EXE_awtrix"))
        .args(["--profile", "auth", "device", "state"])
        .env("AWTRIX_CONFIG", &path)
        .env_remove("AWTRIX_URL")
        .output()
        .unwrap();
    let _ = Command::new(env!("CARGO_BIN_EXE_awtrix"))
        .args([
            "--profile",
            "auth",
            "--username",
            "cli-user",
            "--password",
            "cli-password",
            "device",
            "state",
        ])
        .env("AWTRIX_CONFIG", &path)
        .env_remove("AWTRIX_URL")
        .output()
        .unwrap();
    worker.join().unwrap();
    let _ = std::fs::remove_file(path);
}

#[test]
fn diagnose_reports_target_origin_for_environment_selection() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        for _ in 0..3 {
            let request = server.recv().unwrap();
            let body = match request.url() {
                "/api/v1/device" => r#"{"boardType":"awtrixng","soc":"esp32"}"#,
                "/api/v1/version" => r#"{"version":"x"}"#,
                _ => r#"{"effects":[]}"#,
            };
            request.respond(Response::from_string(body)).unwrap();
        }
    });
    let output = Command::new(env!("CARGO_BIN_EXE_awtrix"))
        .args(["--json", "device", "diagnose"])
        .env("AWTRIX_URL", url)
        .env_remove("AWTRIX_CONFIG")
        .output()
        .unwrap();
    worker.join().unwrap();
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["target_origin"], "environment");
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
            for _ in 0..3 {
                let mut request = server.recv().unwrap();
                let mut body = String::new();
                request.as_reader().read_to_string(&mut body).unwrap();
                let (status, content) = match request.url() {
                    "/api/v1/system" => (500, "system endpoint unavailable".into()),
                    "/api/v1/version" => (200, r#"{"version":"1.2.3"}"#.into()),
                    "/api/v1/device" => {
                        let (board, soc) = match variant {
                            "esp32" => ("awtrixng", "esp32"),
                            "esp32-s3" => ("awtrixng", "esp32s3"),
                            _ => ("tc002", "esp32s3"),
                        };
                        (
                            200,
                            format!(r#"{{"uptime":42,"boardType":"{board}","soc":"{soc}"}}"#),
                        )
                    }
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
fn identity_variant_uses_only_documented_identity_fields() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        for _ in 0..2 {
            let request = server.recv().unwrap();
            let body = if request.url() == "/api/v1/device" {
                r#"{"boardType":"awtrixng","soc":"esp32","note":"tc002 and esp32s3"}"#
            } else {
                r#"{"version":"x"}"#
            };
            request.respond(Response::from_string(body)).unwrap();
        }
    });
    let output = run(&["--target", &url, "--json", "device", "identity"]);
    worker.join().unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["variant"], "ESP32");
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
    assert!(output.stdout.is_empty());
}

#[test]
fn json_field_selection_error_is_machine_readable_on_stdout() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        let request = server.recv().unwrap();
        request
            .respond(Response::from_string(r#"{"uptime":1}"#))
            .unwrap();
    });
    let output = run(&[
        "--target", &url, "--json", "--fields", "missing", "device", "state",
    ]);
    worker.join().unwrap();
    let error: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(error["error"]["code"], "UNKNOWN_FIELD");
    assert!(String::from_utf8_lossy(&output.stderr).contains("UNKNOWN_FIELD"));
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn describe_refines_capabilities_when_explicit_target_is_given() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        let request = server.recv().unwrap();
        assert_eq!(request.url(), "/api/v1/capabilities");
        request
            .respond(Response::from_string(r#"{"effects":["Fade"]}"#))
            .unwrap();
    });
    let output = run(&["--target", &url, "--json", "describe", "device diagnose"]);
    worker.join().unwrap();
    let description: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(description["capability_source"], "connected-device");
    assert_eq!(description["connected_capabilities"]["effects"][0], "Fade");
    assert!(description["parameters"]["--target"].is_string());
    assert!(description["examples"].is_array());
}

#[test]
fn target_can_be_selected_from_environment() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        let request = server.recv().unwrap();
        assert_eq!(request.url(), "/api/v1/device");
        request
            .respond(Response::from_string(r#"{"uptime":7}"#))
            .unwrap();
    });
    let output = run_with_url_env(&["--json", "device", "state"], &url);
    worker.join().unwrap();
    assert!(output.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["uptime"],
        7
    );
}

#[test]
fn common_http_failures_are_classified_without_response_body_leakage() {
    for status in [403, 404, 422, 500] {
        let server = Server::http("127.0.0.1:0").unwrap();
        let url = format!("http://{}", server.server_addr());
        let worker = thread::spawn(move || {
            let request = server.recv().unwrap();
            request
                .respond(Response::from_string("sensitive server detail").with_status_code(status))
                .unwrap();
        });
        let output = run(&["--target", &url, "--json", "device", "state"]);
        worker.join().unwrap();
        let error: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(output.status.code(), Some(5));
        assert_eq!(error["error"]["code"], "HTTP");
        assert!(!String::from_utf8_lossy(&output.stderr).contains("sensitive server detail"));
    }
}

#[test]
fn unreachable_target_is_a_transport_failure() {
    let output = run(&[
        "--target",
        "http://127.0.0.1:1",
        "--json",
        "device",
        "state",
    ]);
    assert_eq!(output.status.code(), Some(1));
    let error: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(error["error"]["code"], "TRANSPORT");
}

#[test]
fn invalid_cli_arguments_have_machine_error_when_json_requested() {
    let output = run(&["--json", "device", "no-such-action"]);
    assert_eq!(output.status.code(), Some(2));
    let error: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(error["error"]["code"], "ARGUMENT");
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
