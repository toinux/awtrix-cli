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
    assert_eq!(
        String::from_utf8_lossy(&get.stdout),
        "# @name demo\nprint('hi')\n"
    );
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
