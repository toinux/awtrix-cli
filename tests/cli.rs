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
        for (expected, status) in [
            ("Basic cHJvZmlsZS11c2VyOnByb2ZpbGUtcGFzc3dvcmQ=", 401),
            ("Basic Y2xpLXVzZXI6Y2xpLXBhc3N3b3Jk", 200),
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
            request
                .respond(Response::from_string("{}").with_status_code(status))
                .unwrap();
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
    let rejected = Command::new(env!("CARGO_BIN_EXE_awtrix"))
        .args(["--profile", "auth", "device", "state"])
        .env("AWTRIX_CONFIG", &path)
        .env_remove("AWTRIX_URL")
        .output()
        .unwrap();
    let rejection_text = format!(
        "{}{}",
        String::from_utf8_lossy(&rejected.stdout),
        String::from_utf8_lossy(&rejected.stderr)
    );
    assert!(!rejection_text.contains("profile-password"));
    let overridden = Command::new(env!("CARGO_BIN_EXE_awtrix"))
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
    assert!(overridden.status.success());
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
fn profile_descriptions_cover_each_operation_and_output_schema() {
    for action in ["add", "update", "list", "show", "set-default", "delete"] {
        let topic = format!("profile {action}");
        let output = run(&["--json", "describe", &topic]);
        assert!(
            output.status.success(),
            "{action}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["command"], format!("profile {action}"));
        assert!(!value["output_fields"].as_array().unwrap().is_empty());
        assert!(!value["examples"].as_array().unwrap().is_empty());
    }
}

#[test]
fn explicit_url_precedes_environment_and_profile_then_environment_precedes_profile() {
    let dir = std::env::temp_dir().join(format!("awtrix-priority-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let config = dir.join("config.json");
    let add = |name: &str, url: &str| {
        run_with_config(&["profile", "add", name, "--target", url], &config)
    };
    let a = Server::http("127.0.0.1:0").unwrap();
    let a_url = format!("http://{}", a.server_addr());
    let b = Server::http("127.0.0.1:0").unwrap();
    let b_url = format!("http://{}", b.server_addr());
    let c = Server::http("127.0.0.1:0").unwrap();
    let c_url = format!("http://{}", c.server_addr());
    assert!(add("default", &a_url).status.success());
    assert!(add("chosen", &b_url).status.success());
    assert!(
        run_with_config(&["profile", "set-default", "default"], &config)
            .status
            .success()
    );
    let wa = thread::spawn(move || {
        for _ in 0..4 {
            let r = a.recv().unwrap();
            let body = match r.url() {
                "/api/v1/device" => r#"{"boardType":"awtrixng","soc":"esp32"}"#,
                "/api/v1/version" => r#"{"version":"x"}"#,
                _ => r#"{"effects":[]}"#,
            };
            r.respond(Response::from_string(body)).unwrap();
        }
    });
    let wb = thread::spawn(move || {
        let r = b.recv().unwrap();
        r.respond(Response::from_string("{}")).unwrap();
    });
    let wc = thread::spawn(move || {
        let r = c.recv().unwrap();
        r.respond(Response::from_string("{}")).unwrap();
    });
    let explicit = Command::new(env!("CARGO_BIN_EXE_awtrix"))
        .args(["--target", &c_url, "--profile", "chosen", "device", "state"])
        .env("AWTRIX_URL", &a_url)
        .env("AWTRIX_CONFIG", &config)
        .output()
        .unwrap();
    assert!(explicit.status.success());
    let selected = Command::new(env!("CARGO_BIN_EXE_awtrix"))
        .args(["--profile", "chosen", "device", "state"])
        .env("AWTRIX_CONFIG", &config)
        .env_remove("AWTRIX_URL")
        .output()
        .unwrap();
    assert!(selected.status.success());
    let environment = Command::new(env!("CARGO_BIN_EXE_awtrix"))
        .args(["device", "state"])
        .env("AWTRIX_URL", &a_url)
        .env("AWTRIX_CONFIG", &config)
        .output()
        .unwrap();
    assert!(environment.status.success());
    let default = Command::new(env!("CARGO_BIN_EXE_awtrix"))
        .args(["--json", "device", "diagnose"])
        .env("AWTRIX_CONFIG", &config)
        .env_remove("AWTRIX_URL")
        .env_remove("AWTRIX_PROFILE")
        .output()
        .unwrap();
    assert!(
        default.status.success(),
        "{}",
        String::from_utf8_lossy(&default.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&default.stdout).unwrap()["target_origin"],
        "default-profile"
    );
    wa.join().unwrap();
    wb.join().unwrap();
    wc.join().unwrap();
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn environment_credentials_override_profile_and_profile_credentials_do_not_follow_other_urls() {
    let dir = std::env::temp_dir().join(format!("awtrix-creds-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let config = dir.join("config.json");
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        for expected in [Some("Basic ZW52LXVzZXI6ZW52LXBhc3M="), None, None] {
            let request = server.recv().unwrap();
            let auth = request
                .headers()
                .iter()
                .find(|h| h.field.equiv("Authorization"))
                .map(|h| h.value.as_str());
            assert_eq!(auth, expected);
            request.respond(Response::from_string("{}")).unwrap();
        }
    });
    let added = run_with_config(
        &[
            "profile",
            "add",
            "secret-profile",
            "--target",
            &url,
            "--username",
            "stored-user",
            "--password",
            "stored-password",
        ],
        &config,
    );
    assert!(added.status.success());
    let selected = Command::new(env!("CARGO_BIN_EXE_awtrix"))
        .args(["--profile", "secret-profile", "device", "state"])
        .env("AWTRIX_CONFIG", &config)
        .env("AWTRIX_USERNAME", "env-user")
        .env("AWTRIX_PASSWORD", "env-pass")
        .env_remove("AWTRIX_URL")
        .output()
        .unwrap();
    assert!(selected.status.success());
    let explicit = Command::new(env!("CARGO_BIN_EXE_awtrix"))
        .args(["--target", &url, "device", "state"])
        .env("AWTRIX_CONFIG", &config)
        .env_remove("AWTRIX_USERNAME")
        .env_remove("AWTRIX_PASSWORD")
        .env_remove("AWTRIX_URL")
        .output()
        .unwrap();
    assert!(explicit.status.success());
    let env_target = Command::new(env!("CARGO_BIN_EXE_awtrix"))
        .args(["device", "state"])
        .env("AWTRIX_CONFIG", &config)
        .env("AWTRIX_URL", &url)
        .env_remove("AWTRIX_USERNAME")
        .env_remove("AWTRIX_PASSWORD")
        .output()
        .unwrap();
    assert!(env_target.status.success());
    worker.join().unwrap();
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn missing_profile_and_credential_bearing_urls_fail_without_secret_disclosure() {
    let dir = std::env::temp_dir().join(format!("awtrix-missing-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let config = dir.join("config.json");
    let missing = Command::new(env!("CARGO_BIN_EXE_awtrix"))
        .args(["--json", "--profile", "absent", "device", "state"])
        .env("AWTRIX_CONFIG", &config)
        .env_remove("AWTRIX_URL")
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(1));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&missing.stdout).unwrap()["error"]["code"],
        "PROFILE_NOT_FOUND"
    );
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        let r = server.recv().unwrap();
        r.respond(Response::from_string("{}")).unwrap();
    });
    let explicit = Command::new(env!("CARGO_BIN_EXE_awtrix"))
        .args(["--target", &url, "--profile", "absent", "device", "state"])
        .env("AWTRIX_CONFIG", &config)
        .env_remove("AWTRIX_URL")
        .output()
        .unwrap();
    assert!(explicit.status.success());
    worker.join().unwrap();
    let bad = run_with_config(
        &[
            "--json",
            "profile",
            "add",
            "bad",
            "--target",
            "http://private-user:private-secret@localhost",
        ],
        &config,
    );
    let all = format!(
        "{}{}",
        String::from_utf8_lossy(&bad.stdout),
        String::from_utf8_lossy(&bad.stderr)
    );
    assert_eq!(bad.status.code(), Some(2));
    assert!(!all.contains("private-user"));
    assert!(!all.contains("private-secret"));
    let _ = std::fs::remove_dir_all(dir);
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

#[test]
fn script_get_preserves_raw_source_and_script_put_reports_berry_error() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        let request = server.recv().unwrap();
        assert_eq!(request.url(), "/api/v1/apps/script/demo");
        request
            .respond(Response::from_string("# @name demo\nprint('hi')\n"))
            .unwrap();
        let mut request = server.recv().unwrap();
        assert_eq!(request.method(), &tiny_http::Method::Put);
        let mut body = String::new();
        request.as_reader().read_to_string(&mut body).unwrap();
        assert_eq!(body, "print('hi')\n");
        request
            .respond(Response::from_string(
                r#"{"ok":true,"name":"demo","error":{"message":"compile failed","line":2}}"#,
            ))
            .unwrap();
    });
    let get = run(&["--target", &url, "--json", "script", "get", "demo"]);
    assert!(get.status.success());
    let source_json: serde_json::Value = serde_json::from_slice(&get.stdout).unwrap();
    assert_eq!(source_json["source"], "# @name demo\nprint('hi')\n");
    let put = run(&[
        "--target",
        &url,
        "--json",
        "script",
        "deploy",
        "demo",
        "--source",
        "print('hi')\n",
        "--force",
    ]);
    worker.join().unwrap();
    assert_eq!(put.status.code(), Some(1));
    let result: serde_json::Value = serde_json::from_slice(&put.stdout).unwrap();
    assert_eq!(result["error"]["code"], "BERRY_ERROR");
}

#[test]
fn script_verify_detects_runtime_error_with_cursor_diagnostics_and_nonzero_exit() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        for (route, body) in [
            ("/api/v1/system", r#"{"scriptingEnabled":true}"#),
            (
                "/api/v1/apps",
                r#"[{"name":"demo","origin":"script","enabled":true,"error":null}]"#,
            ),
            (
                "/api/v1/logs?after=4",
                r#"{"next":5,"lines":["demo failed"]}"#,
            ),
            (
                "/api/v1/apps",
                r#"[{"name":"demo","origin":"script","enabled":true,"error":{"message":"late failure","line":12}}]"#,
            ),
        ] {
            let request = server.recv().unwrap();
            assert_eq!(request.url(), route);
            request.respond(Response::from_string(body)).unwrap();
        }
    });
    let output = run(&[
        "--target",
        &url,
        "--json",
        "script",
        "verify",
        "demo",
        "--after",
        "4",
        "--duration-secs",
        "1",
        "--interval-ms",
        "1",
    ]);
    worker.join().unwrap();
    assert_eq!(output.status.code(), Some(1));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["start_verified"], true);
    assert_eq!(value["logs"]["next"], 5);
    assert_eq!(value["runtime_error"]["line"], 12);
    assert_eq!(value["runtime_state"]["error"]["message"], "late failure");
    assert!(String::from_utf8_lossy(&output.stderr).contains("BERRY_ERROR"));
}

#[test]
fn script_enable_uses_bare_boolean_and_script_state_exposes_only_observed_error_fields() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        let mut request = server.recv().unwrap();
        assert_eq!(request.method(), &tiny_http::Method::Put);
        assert_eq!(request.url(), "/api/v1/apps/demo/enabled");
        let mut body = String::new();
        request.as_reader().read_to_string(&mut body).unwrap();
        assert_eq!(body, "true");
        request
            .respond(Response::from_string(r#"{"ok":true}"#))
            .unwrap();
        let request = server.recv().unwrap();
        assert_eq!(request.url(), "/api/v1/apps");
        request.respond(Response::from_string(r#"[{"name":"demo","origin":"script","enabled":true,"error":{"message":"bad","line":4}}]"#)).unwrap();
    });
    let enabled = run(&["--target", &url, "--json", "script", "enable", "demo"]);
    assert!(enabled.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&enabled.stdout).unwrap()
            ["runtime_success_guaranteed"],
        false
    );
    let state = run(&["--target", &url, "--json", "script", "state"]);
    worker.join().unwrap();
    let state: serde_json::Value = serde_json::from_slice(&state.stdout).unwrap();
    assert_eq!(state["scripts"][0]["error_message"], "bad");
    assert_eq!(state["scripts"][0]["error_line"], 4);
    assert!(state["scripts"][0].get("error_hook").unwrap().is_null());
}

#[test]
fn lifecycle_disable_delete_config_and_data_use_official_routes() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        for (method, route, expected_body, response) in [
            (
                tiny_http::Method::Put,
                "/api/v1/apps/demo/enabled",
                "false",
                r#"{"ok":true}"#,
            ),
            (
                tiny_http::Method::Delete,
                "/api/v1/apps/demo",
                "",
                r#"{"ok":true}"#,
            ),
            (
                tiny_http::Method::Get,
                "/api/v1/apps/demo/config",
                "",
                r#"{"name":"demo","fields":[],"warnings":[]}"#,
            ),
            (
                tiny_http::Method::Patch,
                "/api/v1/apps/demo/config",
                "{}",
                r#"{"ok":true,"name":"demo","error":null}"#,
            ),
            (
                tiny_http::Method::Get,
                "/api/v1/apps/demo/data",
                "",
                r#"{"counter":3}"#,
            ),
        ] {
            let mut request = server.recv().unwrap();
            assert_eq!(request.method(), &method);
            assert_eq!(request.url(), route);
            let mut body = String::new();
            request.as_reader().read_to_string(&mut body).unwrap();
            assert_eq!(body, expected_body);
            request.respond(Response::from_string(response)).unwrap();
        }
    });
    for args in [
        vec!["--target", &url, "--json", "script", "disable", "demo"],
        vec!["--target", &url, "--json", "script", "delete", "demo"],
        vec!["--target", &url, "--json", "script", "config-get", "demo"],
        vec![
            "--target",
            &url,
            "--json",
            "script",
            "config-put",
            "demo",
            "--values",
            "{}",
        ],
        vec!["--target", &url, "--json", "script", "data", "demo"],
    ] {
        let output = run(&args);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
    worker.join().unwrap();
}

#[test]
fn enable_507_reports_official_applied_but_not_persisted_state_without_body_leakage() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        let request = server.recv().unwrap();
        request
            .respond(Response::from_string("private device detail").with_status_code(507))
            .unwrap();
    });
    let output = run(&["--target", &url, "--json", "script", "disable", "demo"]);
    worker.join().unwrap();
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["error"]["code"], "APPLIED_NOT_SAVED");
    assert!(result["error"]["message"]
        .as_str()
        .unwrap()
        .contains("applied but not persisted"));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("private device detail"));
}

#[test]
fn script_state_rejects_invalid_inventory_and_preserves_absent_error_as_null() {
    for (body, expected) in [
        ("null", Some("INVALID_RESPONSE")),
        (
            r#"[{"name":"demo","origin":"script","enabled":true}]"#,
            None,
        ),
    ] {
        let server = Server::http("127.0.0.1:0").unwrap();
        let url = format!("http://{}", server.server_addr());
        let body = body.to_owned();
        let worker = thread::spawn(move || {
            server
                .recv()
                .unwrap()
                .respond(Response::from_string(body))
                .unwrap()
        });
        let output = run(&["--target", &url, "--json", "script", "state"]);
        worker.join().unwrap();
        if let Some(code) = expected {
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["error"]
                    ["code"],
                code
            );
        } else {
            let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            assert!(result["scripts"][0]["error_message"].is_null());
            assert!(result["scripts"][0]["error_line"].is_null());
            assert!(result["scripts"][0]["error_hook"].is_null());
        }
    }
}

#[test]
fn script_lifecycle_incompatible_http_failure_is_structured_without_body_leakage() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        server
            .recv()
            .unwrap()
            .respond(Response::from_string("private").with_status_code(503))
            .unwrap()
    });
    let output = run(&["--target", &url, "--json", "script", "config-get", "demo"]);
    worker.join().unwrap();
    let error: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(error["error"]["code"], "HTTP");
    assert!(!String::from_utf8_lossy(&output.stdout).contains("private"));
}

#[test]
fn script_deploy_uses_atomic_expected_source_route_and_does_not_pre_read() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        let request = server.recv().unwrap();
        assert_eq!(request.url(), "/api/v1/capabilities");
        request
            .respond(Response::from_string(r#"{"scriptUpdates":true}"#))
            .unwrap();
        let mut request = server.recv().unwrap();
        assert_eq!(request.url(), "/api/v1/apps/script-update/demo");
        let mut body = String::new();
        request.as_reader().read_to_string(&mut body).unwrap();
        let payload: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(payload["expected_source"], "old source");
        assert_eq!(payload["source"], "new source");
        request
            .respond(Response::from_string(r#"{"ok":true,"error":null}"#))
            .unwrap();
        let request = server.recv().unwrap();
        assert_eq!(request.url(), "/api/v1/system");
        request
            .respond(Response::from_string(r#"{"scriptingEnabled":true}"#))
            .unwrap();
        let request = server.recv().unwrap();
        assert_eq!(request.url(), "/api/v1/apps");
        request
            .respond(Response::from_string(
                r#"[{"name":"demo","origin":"script","enabled":true,"error":null}]"#,
            ))
            .unwrap();
    });
    let output = run(&[
        "--target",
        &url,
        "--json",
        "script",
        "deploy",
        "demo",
        "--source",
        "new source",
        "--expected-source",
        "old source",
    ]);
    worker.join().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["source_saved"], true);
    assert_eq!(result["start_verified"], true);
}

#[test]
fn script_deploy_surfaces_conflict_without_fallback_or_overwrite() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        let request = server.recv().unwrap();
        request
            .respond(Response::from_string(r#"{"scriptUpdates":true}"#))
            .unwrap();
        let request = server.recv().unwrap();
        assert_eq!(request.url(), "/api/v1/apps/script-update/demo");
        request
            .respond(Response::from_string(r#"{"code":"scriptChanged"}"#).with_status_code(409))
            .unwrap();
    });
    let output = run(&[
        "--target",
        &url,
        "--json",
        "script",
        "deploy",
        "demo",
        "--source",
        "new",
        "--expected-source",
        "original",
    ]);
    worker.join().unwrap();
    assert_eq!(output.status.code(), Some(1));
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["error"]["code"], "CONFLICT");
}

#[test]
fn script_deploy_without_update_capability_does_not_write() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        let request = server.recv().unwrap();
        request
            .respond(Response::from_string(r#"{"other":true}"#))
            .unwrap();
    });
    let output = run(&[
        "--target",
        &url,
        "--json",
        "script",
        "deploy",
        "demo",
        "--source",
        "new",
        "--expected-source",
        "original",
    ]);
    worker.join().unwrap();
    assert_eq!(output.status.code(), Some(6));
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["error"]["code"], "PROTECTION_UNAVAILABLE");
}

#[test]
fn script_create_sends_null_reference_and_verifies_state_only_after_inspection() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        let request = server.recv().unwrap();
        request
            .respond(Response::from_string(r#"{"scriptUpdates":true}"#))
            .unwrap();
        let mut request = server.recv().unwrap();
        let mut body = String::new();
        request.as_reader().read_to_string(&mut body).unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&body).unwrap()["expected_source"],
            serde_json::Value::Null
        );
        request
            .respond(Response::from_string(r#"{"ok":true,"error":null}"#))
            .unwrap();
        server
            .recv()
            .unwrap()
            .respond(Response::from_string(r#"{"scriptingEnabled":false}"#))
            .unwrap();
    });
    let output = run(&[
        "--target", &url, "--json", "script", "deploy", "demo", "--source", "new", "--create",
    ]);
    worker.join().unwrap();
    assert!(output.status.success());
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["source_saved"], true);
    assert_eq!(result["start_verified"], false);
    assert_eq!(result["execution_state"], "unknown");
}

#[test]
fn forced_script_deploy_stays_raw_and_reports_weak_guarantee_for_large_source() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let expected = "x".repeat(9000);
    let observed = expected.clone();
    let worker = thread::spawn(move || {
        let mut request = server.recv().unwrap();
        assert_eq!(request.url(), "/api/v1/apps/script/demo");
        assert!(request
            .headers()
            .iter()
            .any(|header| header.field.equiv("Content-Type")
                && header.value.as_str().starts_with("text/plain")));
        let mut body = String::new();
        request.as_reader().read_to_string(&mut body).unwrap();
        assert_eq!(body, observed);
        request
            .respond(Response::from_string(r#"{"ok":true,"error":null}"#))
            .unwrap();
    });
    let output = run(&[
        "--target", &url, "--json", "script", "deploy", "demo", "--source", &expected, "--force",
    ]);
    worker.join().unwrap();
    assert!(output.status.success());
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["start_verified"], false);
    assert_eq!(result["execution_state"], "unknown");
}

#[test]
fn script_get_non_json_is_exact_raw_source() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        server
            .recv()
            .unwrap()
            .respond(Response::from_string("raw\nsource\n"))
            .unwrap()
    });
    let output = run(&["--target", &url, "script", "get", "demo"]);
    worker.join().unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, b"raw\nsource\n");
}

#[test]
fn successful_conditional_response_with_setup_error_is_operational_failure() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        server
            .recv()
            .unwrap()
            .respond(Response::from_string(r#"{"scriptUpdates":true}"#))
            .unwrap();
        server
            .recv()
            .unwrap()
            .respond(
                Response::from_string(r#"{"expected_source":"old","source":"new"}"#)
                    .with_status_code(422),
            )
            .unwrap();
    });
    let output = run(&[
        "--target",
        &url,
        "--json",
        "script",
        "deploy",
        "demo",
        "--source",
        "new",
        "--expected-source",
        "old",
    ]);
    worker.join().unwrap();
    assert_eq!(output.status.code(), Some(5));
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["error"]["code"], "HTTP");
}

#[test]
fn script_deploy_reads_utf8_source_file_verbatim() {
    let path = std::env::temp_dir().join(format!("awtrix-source-{}.be", std::process::id()));
    std::fs::write(&path, "# café\nprint('ok')\n").unwrap();
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        let mut request = server.recv().unwrap();
        assert_eq!(request.url(), "/api/v1/apps/script/demo");
        let mut body = String::new();
        request.as_reader().read_to_string(&mut body).unwrap();
        assert_eq!(body, "# café\nprint('ok')\n");
        request
            .respond(Response::from_string(r#"{"ok":true,"error":null}"#))
            .unwrap();
    });
    let path_string = path.to_string_lossy().into_owned();
    let output = run(&[
        "--target",
        &url,
        "--json",
        "script",
        "deploy",
        "demo",
        "--file",
        &path_string,
        "--force",
    ]);
    worker.join().unwrap();
    let _ = std::fs::remove_file(path);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["execution_state"], "unknown");
}

#[test]
fn force_cannot_be_combined_with_create_or_expected_source() {
    for conditional in ["--create", "--expected-source"] {
        let mut args = vec![
            "--target",
            "http://127.0.0.1:1",
            "--json",
            "script",
            "deploy",
            "demo",
            "--source",
            "new source",
            "--force",
        ];
        args.push(conditional);
        if conditional == "--expected-source" {
            args.push("original source");
        }
        let output = run(&args);
        assert_eq!(output.status.code(), Some(2));
        let error: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(error["error"]["code"], "ARGUMENT");
        assert!(!String::from_utf8_lossy(&output.stderr).contains("could not reach"));
    }
}

#[test]
fn logs_follow_advances_api_cursor_and_emits_jsonl_without_duplicate_records() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        for (expected, body) in [
            (
                "/api/v1/logs?after=0",
                r#"{"next":4,"lines":["[ 1s] first"]}"#,
            ),
            (
                "/api/v1/logs?after=4",
                r#"{"next":5,"lines":["[ 2s] second"]}"#,
            ),
            ("/api/v1/logs?after=5", r#"{"next":5,"lines":[]}"#),
        ] {
            let request = server.recv().unwrap();
            assert_eq!(request.url(), expected);
            request.respond(Response::from_string(body)).unwrap();
        }
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_millis(1300) {
            if let Some(request) = server.recv_timeout(Duration::from_millis(50)).unwrap() {
                assert_eq!(request.url(), "/api/v1/logs?after=5");
                request
                    .respond(Response::from_string(r#"{"next":5,"lines":[]}"#))
                    .unwrap();
            }
        }
    });
    let output = run(&[
        "--target",
        &url,
        "--json",
        "logs",
        "follow",
        "--after",
        "0",
        "--interval-ms",
        "10",
        "--duration-secs",
        "1",
    ]);
    worker.join().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let records: Vec<serde_json::Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(records.len(), 3);
    assert_eq!(records[0]["line"], "[ 1s] first");
    assert_eq!(records[1]["line"], "[ 2s] second");
    assert_eq!(records[2]["type"], "end");
}

#[test]
fn logs_read_reports_empty_stream_and_remote_failure() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        let request = server.recv().unwrap();
        assert_eq!(request.url(), "/api/v1/logs?after=7");
        request
            .respond(Response::from_string(r#"{"next":7,"lines":[]}"#))
            .unwrap();
        let request = server.recv().unwrap();
        request
            .respond(Response::from_string("private detail").with_status_code(500))
            .unwrap();
    });
    let empty = run(&["--target", &url, "--json", "logs", "read", "--after", "7"]);
    assert!(empty.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&empty.stdout).unwrap()["lines"],
        serde_json::json!([])
    );
    let failure = run(&["--target", &url, "--json", "logs", "read"]);
    worker.join().unwrap();
    assert_eq!(failure.status.code(), Some(5));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&failure.stdout).unwrap()["error"]["code"],
        "HTTP"
    );
    assert!(!String::from_utf8_lossy(&failure.stdout).contains("private detail"));
}

#[test]
fn logs_follow_request_timeout_is_clamped_to_remaining_duration() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        let request = server.recv().unwrap();
        thread::sleep(Duration::from_millis(1500));
        let _ = request.respond(Response::from_string(r#"{"next":0,"lines":[]}"#));
    });
    let start = std::time::Instant::now();
    let output = run(&[
        "--target",
        &url,
        "--timeout",
        "3000",
        "--json",
        "logs",
        "follow",
        "--duration-secs",
        "1",
    ]);
    let elapsed = start.elapsed();
    worker.join().unwrap();
    assert_eq!(output.status.code(), Some(4));
    assert!(
        elapsed < Duration::from_millis(1400),
        "request exceeded follow deadline: {elapsed:?}"
    );
}

#[test]
fn logs_follow_partial_failure_emits_error_and_resume_end_records() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        server
            .recv()
            .unwrap()
            .respond(Response::from_string(
                r#"{"next":8,"lines":["script: started"]}"#,
            ))
            .unwrap();
        server
            .recv()
            .unwrap()
            .respond(Response::from_string("secret").with_status_code(500))
            .unwrap();
    });
    let output = run(&[
        "--target",
        &url,
        "--json",
        "logs",
        "follow",
        "--interval-ms",
        "1",
        "--duration-secs",
        "1",
    ]);
    worker.join().unwrap();
    assert_eq!(output.status.code(), Some(5));
    let rows: Vec<serde_json::Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(rows[0]["type"], "log");
    assert_eq!(rows[1]["type"], "error");
    assert_eq!(rows[1]["next"], 8);
    assert_eq!(rows[2]["type"], "end");
    assert_eq!(rows[2]["next"], 8);
    assert!(!rows.iter().any(|row| row.to_string().contains("secret")));
}

#[test]
fn logs_follow_rejects_fields_before_http_request() {
    let output = run(&[
        "--target",
        "http://127.0.0.1:1",
        "--json",
        "--fields",
        "next",
        "logs",
        "follow",
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["error"]["code"],
        "ARGUMENT"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("--fields"));
}

#[test]
fn logs_follow_filters_by_literal_text_and_ctrl_c_emits_interrupted_end() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        let request = server.recv().unwrap();
        request
            .respond(Response::from_string(
                r#"{"next":2,"lines":["weather: ready","other: ignored"]}"#,
            ))
            .unwrap();
        while let Some(request) = server.recv_timeout(Duration::from_millis(100)).unwrap() {
            request
                .respond(Response::from_string(r#"{"next":2,"lines":[]}"#))
                .unwrap();
        }
    });
    let child = Command::new(env!("CARGO_BIN_EXE_awtrix"))
        .args([
            "--target",
            &url,
            "--json",
            "logs",
            "follow",
            "--script",
            "weather",
            "--interval-ms",
            "60000",
            "--duration-secs",
            "3600",
        ])
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    thread::sleep(Duration::from_millis(100));
    let signaled = Command::new("kill")
        .args(["-INT", &child.id().to_string()])
        .status()
        .unwrap();
    assert!(signaled.success());
    let output = child.wait_with_output().unwrap();
    worker.join().unwrap();
    assert!(output.status.success());
    let rows: Vec<serde_json::Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["line"], "weather: ready");
    assert_eq!(rows[1]["type"], "end");
    assert_eq!(rows[1]["interrupted"], true);
}

#[test]
fn log_descriptions_distinguish_read_and_follow_schemas_and_global_parameters() {
    let read = run(&["--json", "describe", "logs read"]);
    let follow = run(&["--json", "describe", "logs follow"]);
    let read: serde_json::Value = serde_json::from_slice(&read.stdout).unwrap();
    let follow: serde_json::Value = serde_json::from_slice(&follow.stdout).unwrap();
    for key in [
        "--target",
        "--profile",
        "--username",
        "--password",
        "--timeout",
        "--json",
        "--fields",
    ] {
        assert!(read["parameters"][key].is_string());
        assert!(follow["parameters"][key].is_string());
    }
    assert!(read["outputs"]["lines"].is_array());
    assert!(follow["outputs"]["records"].is_array());
    assert!(follow["output_fields"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("code")));
}

#[test]
fn screen_capture_writes_rgb_png_and_reports_only_location_and_dimensions() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        let request = server.recv().unwrap();
        assert_eq!(request.url(), "/api/v1/display/screen");
        request
            .respond(Response::from_string(
                r#"{"width":2,"height":1,"pixels":[16711681,131844]}"#,
            ))
            .unwrap();
    });
    let path = std::env::temp_dir().join(format!("awtrix-screen-{}.png", std::process::id()));
    let path_arg = path.to_string_lossy().into_owned();
    let output = run(&[
        "--target", &url, "--json", "screen", "capture", "--output", &path_arg,
    ]);
    worker.join().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["format"], "png");
    assert_eq!(result["width"], 2);
    assert_eq!(result["height"], 1);
    assert!(result.get("pixels").is_none());
    let decoder = png::Decoder::new(std::fs::File::open(&path).unwrap());
    let mut reader = decoder.read_info().unwrap();
    assert_eq!((reader.info().width, reader.info().height), (2, 1));
    let mut decoded = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut decoded).unwrap();
    assert_eq!(info.color_type, png::ColorType::Rgb);
    assert_eq!(&decoded[..info.buffer_size()], &[255, 0, 1, 2, 3, 4]);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn screen_capture_accepts_dynamic_dimensions_and_rejects_inconsistent_or_oversized_frames() {
    for (frame, code) in [
        (
            r#"{"width":3,"height":2,"pixels":[0,65793,131586,197379,263172,328965]}"#,
            None,
        ),
        (
            r#"{"width":2,"height":2,"pixels":[0]}"#,
            Some("INVALID_RESPONSE"),
        ),
        (
            r#"{"width":18446744073709551615,"height":2,"pixels":[]}"#,
            Some("INVALID_RESPONSE"),
        ),
        (
            r#"{"width":1,"height":1,"pixels":[16777216]}"#,
            Some("INVALID_RESPONSE"),
        ),
        (
            r#"{"width":0,"height":1,"pixels":[]}"#,
            Some("INVALID_RESPONSE"),
        ),
        (
            r#"{"width":-1,"height":1,"pixels":[]}"#,
            Some("INVALID_RESPONSE"),
        ),
    ] {
        let server = Server::http("127.0.0.1:0").unwrap();
        let url = format!("http://{}", server.server_addr());
        let frame = frame.to_owned();
        let worker = thread::spawn(move || {
            server
                .recv()
                .unwrap()
                .respond(Response::from_string(frame))
                .unwrap()
        });
        let path =
            std::env::temp_dir().join(format!("awtrix-screen-variant-{}.png", std::process::id()));
        let path_arg = path.to_string_lossy().into_owned();
        let output = run(&[
            "--target", &url, "--json", "screen", "capture", "--output", &path_arg,
        ]);
        worker.join().unwrap();
        if let Some(code) = code {
            assert_eq!(output.status.code(), Some(1));
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["error"]
                    ["code"],
                code
            );
        } else {
            assert!(output.status.success());
            let decoder = png::Decoder::new(std::fs::File::open(&path).unwrap());
            let reader = decoder.read_info().unwrap();
            assert_eq!((reader.info().width, reader.info().height), (3, 2));
            std::fs::remove_file(&path).unwrap();
        }
    }
}

#[test]
fn screen_descriptions_and_file_failures_are_machine_readable() {
    let described = run(&["--json", "describe", "screen"]);
    let schema: serde_json::Value = serde_json::from_slice(&described.stdout).unwrap();
    assert!(schema["parameters"]["--output"].is_string());
    assert_eq!(schema["outputs"][0], "path, format, width, height");
    let help = run(&["screen", "capture", "--help"]);
    assert!(String::from_utf8_lossy(&help.stdout).contains("physical brightness"));

    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        server
            .recv()
            .unwrap()
            .respond(Response::from_string(
                r#"{"width":1,"height":1,"pixels":[0]}"#,
            ))
            .unwrap()
    });
    let output = run(&[
        "--target",
        &url,
        "--json",
        "screen",
        "capture",
        "--output",
        "/no/such/awtrix/capture.png",
    ]);
    worker.join().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["error"]["code"],
        "FILE_WRITE"
    );
}

#[test]
fn screen_capture_handles_representative_awtrix_matrix_sizes() {
    for (width, height) in [(32, 8), (52, 16)] {
        let pixels = vec![0x12_34_56u32; width * height];
        let body = serde_json::json!({"width":width,"height":height,"pixels":pixels}).to_string();
        let server = Server::http("127.0.0.1:0").unwrap();
        let url = format!("http://{}", server.server_addr());
        let worker = thread::spawn(move || {
            server
                .recv()
                .unwrap()
                .respond(Response::from_string(body))
                .unwrap()
        });
        let path = std::env::temp_dir().join(format!(
            "awtrix-screen-{}x{}-{}.png",
            width,
            height,
            std::process::id()
        ));
        let path_arg = path.to_string_lossy().into_owned();
        let output = run(&[
            "--target", &url, "--json", "screen", "capture", "--output", &path_arg,
        ]);
        worker.join().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let mut reader = png::Decoder::new(std::fs::File::open(&path).unwrap())
            .read_info()
            .unwrap();
        assert_eq!(
            (reader.info().width, reader.info().height),
            (width as u32, height as u32)
        );
        let mut decoded = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut decoded).unwrap();
        assert!(decoded[..info.buffer_size()]
            .chunks(3)
            .all(|rgb| rgb == [0x12, 0x34, 0x56]));
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn resource_file_upload_uses_official_multipart_contract_and_checks_icon_magic_locally() {
    let path = std::env::temp_dir().join(format!("awtrix-resource-{}.gif", std::process::id()));
    std::fs::write(&path, b"GIF89a payload").unwrap();
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        let mut request = server.recv().unwrap();
        assert_eq!(request.method(), &tiny_http::Method::Post);
        assert_eq!(request.url(), "/api/v1/files?dir=/ICONS");
        assert!(request
            .headers()
            .iter()
            .any(|h| h.field.equiv("Content-Type")
                && h.value
                    .as_str()
                    .starts_with("multipart/form-data; boundary=")));
        let mut body = String::new();
        request.as_reader().read_to_string(&mut body).unwrap();
        assert!(body.contains("name=\"file\""));
        assert!(body.contains("GIF89a payload"));
        request
            .respond(Response::from_string(r#"{"ok":true}"#))
            .unwrap();
    });
    let path_arg = path.to_string_lossy().into_owned();
    let result = run(&[
        "--target",
        &url,
        "--json",
        "resources",
        "files",
        "upload",
        &path_arg,
    ]);
    worker.join().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    std::fs::write(&path, b"not gif data").unwrap();
    let result = run(&[
        "--target",
        "http://127.0.0.1:1",
        "--json",
        "resources",
        "files",
        "upload",
        &path_arg,
    ]);
    assert_eq!(result.status.code(), Some(1));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&result.stdout).unwrap()["error"]["code"],
        "INVALID_RESOURCE"
    );
    std::fs::remove_file(path).unwrap();
}

#[test]
fn resource_jpeg_upload_accepts_minimal_jpeg_signature_but_rejects_case_changed_extension() {
    let path = std::env::temp_dir().join(format!("awtrix-resource-{}.jpg", std::process::id()));
    // Valid 1x1 JPEG fixture; local preflight checks the signature and AWTRIX owns full decoding.
    use base64::Engine;
    let jpeg = base64::engine::general_purpose::STANDARD.decode("/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAAgGBgcGBQgHBwcJCQgKDBQNDAsLDBkSEw8UHRofHh0aHBwgJC4nICIsIxwcKDcpLDAxNDQ0Hyc5PTgyPC4zNDL/wAALCAABAAEBAREA/8QAFAABAAAAAAAAAAAAAAAAAAAAB//EABQQAQAAAAAAAAAAAAAAAAAAAAD/2gAIAQEAAD8Af3//2Q==").unwrap();
    std::fs::write(&path, &jpeg).unwrap();
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        let mut request = server.recv().unwrap();
        assert_eq!(request.url(), "/api/v1/files?dir=/ICONS");
        let mut body = Vec::new();
        request.as_reader().read_to_end(&mut body).unwrap();
        assert!(body.windows(3).any(|window| window == [0xff, 0xd8, 0xff]));
        request
            .respond(Response::from_string(r#"{"ok":true}"#))
            .unwrap();
    });
    let path_arg = path.to_string_lossy().into_owned();
    let result = run(&[
        "--target",
        &url,
        "--json",
        "resources",
        "files",
        "upload",
        &path_arg,
    ]);
    worker.join().unwrap();
    assert!(result.status.success());
    let upper = path.with_extension("JPG");
    std::fs::rename(&path, &upper).unwrap();
    let upper_arg = upper.to_string_lossy().into_owned();
    let result = run(&[
        "--target",
        "http://127.0.0.1:1",
        "--json",
        "resources",
        "files",
        "upload",
        &upper_arg,
    ]);
    assert_eq!(result.status.code(), Some(1));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&result.stdout).unwrap()["error"]["code"],
        "INVALID_RESOURCE"
    );
    std::fs::remove_file(upper).unwrap();
}

#[test]
fn module_marker_after_executable_berry_is_rejected_before_http() {
    let result = run(&[
        "--target",
        "http://127.0.0.1:1",
        "--json",
        "resources",
        "modules",
        "deploy",
        "helpers",
        "--source",
        "var x = 1\n# @module helpers\n",
    ]);
    assert_eq!(result.status.code(), Some(2));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&result.stdout).unwrap()["error"]["code"],
        "ARGUMENT"
    );
}

#[test]
fn resource_upload_preserves_remote_payload_too_large_and_storage_full_statuses() {
    for status in [413, 507] {
        let path = std::env::temp_dir().join(format!(
            "awtrix-resource-{status}-{}.gif",
            std::process::id()
        ));
        std::fs::write(&path, b"GIF89a payload").unwrap();
        let server = Server::http("127.0.0.1:0").unwrap();
        let url = format!("http://{}", server.server_addr());
        let worker = thread::spawn(move || {
            let request = server.recv().unwrap();
            request
                .respond(Response::from_string("private device detail").with_status_code(status))
                .unwrap();
        });
        let path_arg = path.to_string_lossy().into_owned();
        let result = run(&[
            "--target",
            &url,
            "--json",
            "resources",
            "files",
            "upload",
            &path_arg,
        ]);
        worker.join().unwrap();
        assert_eq!(result.status.code(), Some(5));
        let error: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(error["error"]["code"], "HTTP");
        assert!(!String::from_utf8_lossy(&result.stdout).contains("private device detail"));
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn resource_module_list_uses_official_inventory_without_turning_modules_into_apps() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        let request = server.recv().unwrap();
        assert_eq!(request.url(), "/api/v1/apps");
        request.respond(Response::from_string(r#"[{"name":"script","origin":"script"},{"name":"lib","origin":"module","import":"lib"}]"#)).unwrap();
    });
    let result = run(&["--target", &url, "--json", "resources", "modules", "list"]);
    worker.join().unwrap();
    assert!(result.status.success());
    let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(value["modules"].as_array().unwrap().len(), 1);
    assert_eq!(value["modules"][0]["origin"], "module");
}

#[test]
fn resource_descriptions_document_every_operation_and_unsupported_download() {
    for op in [
        "files list",
        "files upload",
        "files delete",
        "modules list",
        "modules get",
        "modules deploy",
        "modules delete",
    ] {
        let topic = format!("resources {op}");
        let result = run(&["--json", "describe", &topic]);
        assert!(result.status.success(), "{topic}");
        let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        assert!(
            value["examples"].as_array().is_some_and(|a| !a.is_empty()),
            "{topic}"
        );
        assert!(value["outputs"].is_array(), "{topic}");
        assert!(value["output_fields"].is_array(), "{topic}");
        for global in [
            "--target",
            "--profile",
            "--username",
            "--password",
            "--timeout",
            "--json",
            "--fields",
        ] {
            assert!(
                value["parameters"][global].is_string(),
                "{topic} missing {global}"
            );
        }
    }
    let result = run(&["--json", "describe", "resources files download"]);
    let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert!(value["examples"][0].as_str().unwrap().contains("download"));
    assert!(value["limitations"][0]
        .as_str()
        .unwrap()
        .contains("No generic"));
}

#[test]
fn resource_module_get_and_deploy_roundtrip_official_script_source_routes() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        let request = server.recv().unwrap();
        assert_eq!(request.url(), "/api/v1/apps/script/helpers");
        request
            .respond(Response::from_string("# @module helpers\nvar x = 1\n"))
            .unwrap();
        let mut request = server.recv().unwrap();
        assert_eq!(request.method(), &tiny_http::Method::Put);
        assert_eq!(request.url(), "/api/v1/apps/script/helpers");
        let mut body = String::new();
        request.as_reader().read_to_string(&mut body).unwrap();
        assert_eq!(body, "# @module helpers\nvar x = 2\n");
        request
            .respond(Response::from_string(r#"{"ok":true,"error":null}"#))
            .unwrap();
    });
    let get = run(&[
        "--target",
        &url,
        "--json",
        "resources",
        "modules",
        "get",
        "helpers",
    ]);
    assert!(get.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&get.stdout).unwrap()["source"],
        "# @module helpers\nvar x = 1\n"
    );
    let deploy = run(&[
        "--target",
        &url,
        "--json",
        "resources",
        "modules",
        "deploy",
        "helpers",
        "--source",
        "# @module helpers\nvar x = 2\n",
    ]);
    worker.join().unwrap();
    assert!(
        deploy.status.success(),
        "{}",
        String::from_utf8_lossy(&deploy.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&deploy.stdout).unwrap();
    assert_eq!(value["rotation_app"], false);
    assert_eq!(value["references_rewritten"], false);
}

#[test]
fn resource_module_delete_uses_official_app_delete_route() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        let request = server.recv().unwrap();
        assert_eq!(request.method(), &tiny_http::Method::Delete);
        assert_eq!(request.url(), "/api/v1/apps/helpers");
        request
            .respond(Response::from_string(r#"{"ok":true}"#))
            .unwrap();
    });
    let result = run(&[
        "--target",
        &url,
        "--json",
        "resources",
        "modules",
        "delete",
        "helpers",
    ]);
    worker.join().unwrap();
    assert!(result.status.success());
}

#[test]
fn resource_download_reports_unsupported_and_never_guesses_an_http_path() {
    let result = run(&[
        "--target",
        "http://127.0.0.1:1",
        "--json",
        "resources",
        "files",
        "download",
        "/ICONS/a.gif",
        "--output",
        "/tmp/a.gif",
    ]);
    assert_eq!(result.status.code(), Some(1));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&result.stdout).unwrap()["error"]["code"],
        "UNSUPPORTED"
    );
}

#[test]
fn resource_file_list_and_delete_use_documented_query_parameters() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        let request = server.recv().unwrap();
        assert_eq!(request.url(), "/api/v1/files?dir=/PALETTES%20ONE");
        request
            .respond(Response::from_string(
                r#"{"files":[{"name":"warm.json","size":8}],"usedBytes":8,"totalBytes":64}"#,
            ))
            .unwrap();
        let request = server.recv().unwrap();
        assert_eq!(request.method(), &tiny_http::Method::Delete);
        assert_eq!(request.url(), "/api/v1/files?path=/ICONS/a%20b.gif");
        request
            .respond(Response::from_string(r#"{"ok":true}"#))
            .unwrap();
    });
    let list = run(&[
        "--target",
        &url,
        "--json",
        "resources",
        "files",
        "list",
        "--dir",
        "/PALETTES ONE",
    ]);
    assert!(list.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&list.stdout).unwrap()["files"][0]["name"],
        "warm.json"
    );
    let delete = run(&[
        "--target",
        &url,
        "--json",
        "resources",
        "files",
        "delete",
        "/ICONS/a b.gif",
    ]);
    worker.join().unwrap();
    assert!(delete.status.success());
}

#[test]
fn screen_capture_preserves_existing_destination_on_invalid_or_oversized_response() {
    let path = std::env::temp_dir().join(format!("awtrix-preserve-{}.png", std::process::id()));
    for (body, expected) in [
        ("{truncated".to_owned(), "INVALID_RESPONSE"),
        (" ".repeat(42 * 1024 * 1024), "RESPONSE_TOO_LARGE"),
    ] {
        std::fs::write(&path, b"previous valid image").unwrap();
        let server = Server::http("127.0.0.1:0").unwrap();
        let url = format!("http://{}", server.server_addr());
        let worker = thread::spawn(move || {
            server
                .recv()
                .unwrap()
                .respond(Response::from_string(body))
                .unwrap()
        });
        let path_arg = path.to_string_lossy().into_owned();
        let output = run(&[
            "--target", &url, "--json", "screen", "capture", "--output", &path_arg,
        ]);
        worker.join().unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"previous valid image");
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["error"]["code"],
            expected
        );
    }
    std::fs::remove_file(path).unwrap();
}

#[test]
fn screen_capture_http_failures_are_structured_and_do_not_touch_destination() {
    let path = std::env::temp_dir().join(format!("awtrix-network-{}.png", std::process::id()));
    {
        let status = 500;
        let server = Server::http("127.0.0.1:0").unwrap();
        let url = format!("http://{}", server.server_addr());
        let worker = thread::spawn(move || {
            server
                .recv()
                .unwrap()
                .respond(Response::from_string("private detail").with_status_code(status))
                .unwrap()
        });
        std::fs::write(&path, b"keep me").unwrap();
        let output = run(&[
            "--target",
            &url,
            "--json",
            "screen",
            "capture",
            "--output",
            &path.to_string_lossy(),
        ]);
        worker.join().unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["error"]["code"],
            "HTTP"
        );
        assert_eq!(std::fs::read(&path).unwrap(), b"keep me");
    }
    std::fs::remove_file(&path).unwrap();
    let output = run(&[
        "--target",
        "http://127.0.0.1:1",
        "--json",
        "screen",
        "capture",
        "--output",
        "/tmp/never-created-awtrix-screen.png",
    ]);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["error"]["code"],
        "TRANSPORT"
    );
}

#[test]
fn screen_capture_output_failure_preserves_existing_directory() {
    let path = std::env::temp_dir().join(format!("awtrix-screen-dir-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir(&path).unwrap();
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        server
            .recv()
            .unwrap()
            .respond(Response::from_string(
                r#"{"width":1,"height":1,"pixels":[0]}"#,
            ))
            .unwrap()
    });
    let output = run(&[
        "--target",
        &url,
        "--json",
        "screen",
        "capture",
        "--output",
        &path.to_string_lossy(),
    ]);
    worker.join().unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["error"]["code"],
        "FILE_WRITE"
    );
    assert!(path.is_dir());
    std::fs::remove_dir(path).unwrap();
}

#[test]
fn screen_capture_replaces_same_destination_with_new_pixels() {
    let path = std::env::temp_dir().join(format!("awtrix-recapture-{}.png", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        for packed_pixel in [0x112233u32, 0xaabbcc] {
            let body =
                serde_json::json!({"width":1,"height":1,"pixels":[packed_pixel]}).to_string();
            server
                .recv()
                .unwrap()
                .respond(Response::from_string(body))
                .unwrap();
        }
    });
    let output_path = path.to_string_lossy().into_owned();
    for _ in 0..2 {
        let output = run(&[
            "--target",
            &url,
            "--json",
            "screen",
            "capture",
            "--output",
            &output_path,
        ]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    worker.join().unwrap();
    let mut reader = png::Decoder::new(std::fs::File::open(&path).unwrap())
        .read_info()
        .unwrap();
    let mut pixels = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut pixels).unwrap();
    assert_eq!(&pixels[..info.buffer_size()], &[0xaa, 0xbb, 0xcc]);
    std::fs::remove_file(path).unwrap();
}
