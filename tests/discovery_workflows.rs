use serde_json::{json, Value};
use std::{
    process::{Command, Output},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};
use tiny_http::{Response, Server};

struct Mock {
    url: String,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}

impl Mock {
    fn new(app: Value, deployment: Value, capture_status: u16) -> Self {
        Self::with_states(vec![app], deployment, capture_status)
    }

    fn with_states(mut apps: Vec<Value>, deployment: Value, capture_status: u16) -> Self {
        let server = Server::http("127.0.0.1:0").unwrap();
        let url = format!("http://{}", server.server_addr());
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let worker = thread::spawn(move || {
            while !flag.load(Ordering::SeqCst) {
                let Some(request) = server.recv_timeout(Duration::from_millis(50)).unwrap() else {
                    continue;
                };
                let (status, body) = match request.url() {
                    "/api/v1/capabilities" => (200, json!({"scriptUpdates":true})),
                    "/api/v1/apps/script-update/main" | "/api/v1/apps/script/main" => {
                        (200, deployment.clone())
                    }
                    "/api/v1/system" => (200, json!({"scriptingEnabled":true})),
                    "/api/v1/apps" => {
                        let app = if apps.len() > 1 {
                            apps.remove(0)
                        } else {
                            apps[0].clone()
                        };
                        (200, json!([app]))
                    }
                    "/api/v1/display/screen" => (capture_status, json!({})),
                    path if path.starts_with("/api/v1/logs?") => {
                        (200, json!({"next":0,"lines":[]}))
                    }
                    path => panic!("unexpected request {path}"),
                };
                request
                    .respond(Response::from_string(body.to_string()).with_status_code(status))
                    .unwrap();
            }
        });
        Self {
            url,
            stop,
            worker: Some(worker),
        }
    }
}

impl Drop for Mock {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.worker.take().unwrap().join().unwrap();
    }
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_awtrix-cli"))
        .args(args)
        .env("AWTRIX_NO_UPDATE_CHECK", "1")
        .env_remove("AWTRIX_URL")
        .env_remove("AWTRIX_PROFILE")
        .env_remove("AWTRIX_LINUX_BIN")
        .output()
        .unwrap()
}

#[test]
fn conditional_berry_failure_does_not_claim_source_saved() {
    let mock = Mock::new(
        json!({"name":"main","origin":"script","enabled":true,"error":null}),
        json!({"error":{"message":"compile failed"}}),
        200,
    );
    let output = run(&[
        "--json",
        "--target",
        &mock.url,
        "script",
        "deploy",
        "main",
        "--source",
        "invalid",
        "--expected-source",
        "original",
    ]);
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(!output.status.success());
    assert_eq!(result["error"]["code"], "BERRY_ERROR");
    let message = result["error"]["message"].as_str().unwrap();
    assert!(!message.contains("source saved"), "{message}");
    assert!(message.contains("restored"), "{message}");
}

#[test]
fn verify_requires_explicit_null_error_for_start_proof() {
    let mock = Mock::new(
        json!({"name":"main","origin":"script","enabled":true}),
        json!({}),
        200,
    );
    let output = run(&[
        "--json",
        "--target",
        &mock.url,
        "script",
        "verify",
        "main",
        "--duration-secs",
        "1",
    ]);
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["start_verified"], false, "{result}");
    assert!(!output.status.success());
    assert_eq!(result["observed_window"]["complete"], false);
}

#[test]
fn capture_failure_identifies_capture_phase() {
    let mock = Mock::new(
        json!({"name":"main","origin":"script","enabled":true,"error":null}),
        json!({}),
        503,
    );
    let root = tempfile::tempdir().unwrap();
    let capture = root.path().join("capture.png");
    let output = run(&[
        "--json",
        "--target",
        &mock.url,
        "script",
        "verify",
        "main",
        "--duration-secs",
        "1",
        "--capture",
        capture.to_str().unwrap(),
    ]);
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["not_available"]["phase"], "capture", "{result}");
    assert!(!capture.exists());
}

#[test]
fn verify_explicit_null_error_completes_observation() {
    let mock = Mock::new(
        json!({"name":"main","origin":"script","enabled":true,"error":null}),
        json!({}),
        200,
    );
    let output = run(&[
        "--json",
        "--target",
        &mock.url,
        "script",
        "verify",
        "main",
        "--duration-secs",
        "1",
    ]);
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(output.status.success(), "{result}");
    assert_eq!(result["start_verified"], true);
    assert_eq!(result["observed_window"]["complete"], true);
    assert_eq!(result["runtime_success_guaranteed"], false);
}

#[test]
fn verify_missing_error_in_later_poll_is_inconclusive() {
    let mock = Mock::with_states(
        vec![
            json!({"name":"main","origin":"script","enabled":true,"error":null}),
            json!({"name":"main","origin":"script","enabled":true}),
        ],
        json!({}),
        200,
    );
    let output = run(&[
        "--json",
        "--target",
        &mock.url,
        "script",
        "verify",
        "main",
        "--duration-secs",
        "1",
    ]);
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(!output.status.success());
    assert_eq!(result["start_verified"], true); // Initial start evidence remains valid.
    assert_eq!(result["not_available"]["phase"], "state_observation");
    assert_eq!(result["observed_window"]["complete"], false);
}

#[test]
fn raw_berry_failure_preserves_documented_saved_source_diagnostic() {
    let mock = Mock::new(
        json!({}),
        json!({"error":{"message":"compile failed"}}),
        200,
    );
    let output = run(&[
        "--json", "--target", &mock.url, "script", "deploy", "main", "--source", "invalid",
        "--force",
    ]);
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(result["error"]["message"]
        .as_str()
        .unwrap()
        .contains("source saved"));
}

#[test]
fn project_test_discovery_includes_all_assertions_and_workflow() {
    let output = run(&["--json", "describe", "test", "project", "--offline"]);
    assert!(output.status.success());
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    let schema = &result["schema"];
    for field in ["script_active", "script + path + equals", "log_contains"] {
        assert!(schema["[[tests.assertion]]"][field].is_string(), "{schema}");
    }
    assert!(schema["[[tests.assertion]] visual"]["reference"].is_string());
    assert!(schema["[tests]"]["window_secs"]
        .as_str()
        .unwrap()
        .contains("default 10"));
    assert!(result["outputs"]["deployment"].is_string());
    assert!(result["workflow"].as_array().unwrap().len() >= 4);
}

#[test]
fn headless_discovery_is_operation_specific_and_local() {
    for (operation, fields) in [
        (
            "start",
            vec![
                "running",
                "pid",
                "target",
                "data",
                "temporary_data",
                "owned",
            ],
        ),
        (
            "status",
            vec![
                "running",
                "owned",
                "pid",
                "target",
                "data",
                "temporary_data",
            ],
        ),
        ("stop", vec!["stopped", "pid", "data_removed"]),
    ] {
        let output = run(&["--json", "describe", "headless", operation, "--offline"]);
        let result: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(result["output_fields"], json!(fields));
        if operation != "start" {
            assert_eq!(result["parameters"], json!({}));
        }
        assert!(result["ownership_record"]
            .as_str()
            .unwrap()
            .contains("AWTRIX_CONFIG"));
    }
}

#[test]
fn discovered_minimal_manifest_validates_without_a_target() {
    let description = run(&["--json", "describe", "project", "deploy", "--offline"]);
    let description: Value = serde_json::from_slice(&description.stdout).unwrap();
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("src")).unwrap();
    std::fs::write(root.path().join("src/main.ax"), "return nil\n").unwrap();
    let manifest = root.path().join("awtrix.toml");
    std::fs::write(&manifest, description["minimal_manifest"].as_str().unwrap()).unwrap();
    let output = run(&[
        "--json",
        "project",
        "validate",
        "--manifest",
        manifest.to_str().unwrap(),
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn multiple_capture_help_and_offline_discovery_match_capture_cli_contract() {
    let help = run(&["script", "deploy", "--help"]);
    let help = String::from_utf8_lossy(&help.stdout);
    assert!(help.contains("--capture-dir"));
    assert!(help.contains("--capture-count"));
    assert!(help.contains("--capture-timeout-ms"));
    assert!(help.contains("default: 10000"));
    let description = run(&["--json", "describe", "script", "--offline"]);
    let description: Value = serde_json::from_slice(&description.stdout).unwrap();
    assert_eq!(
        description["parameters"]["--capture-dir"],
        "opt-in named captures triggered by new exact target-script markers; cannot combine with --verify-secs"
    );
    assert_eq!(
        description["parameters"]["--capture-timeout-ms"],
        "capture observation bound 1..3600000 milliseconds (default 10000), requires --capture-dir"
    );
    assert!(description["examples"]
        .as_array()
        .unwrap()
        .iter()
        .any(|example| example
            .as_str()
            .unwrap()
            .contains("--capture-dir ./captures")));
    assert_eq!(
        description["parameters"]["--capture-count"],
        "positive PNG count (default 1), requires --capture-dir"
    );
}

#[test]
fn force_does_not_bypass_script_reference_validation() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("main.ax"), "return nil\n").unwrap();
    let manifest = root.path().join("awtrix.toml");
    std::fs::write(
        &manifest,
        "[project]\nname='demo'\nversion='1'\n[[scripts]]\nname='main'\nfile='main.ax'\n",
    )
    .unwrap();
    let output = run(&[
        "--json",
        "--target",
        "http://127.0.0.1:1",
        "project",
        "deploy",
        "--manifest",
        manifest.to_str().unwrap(),
        "--force",
    ]);
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["error"]["code"], "PROJECT_INVALID");
    assert!(result["error"]["message"]
        .as_str()
        .unwrap()
        .contains("create=true or expected_source_file"));
}

#[test]
fn resource_help_exposes_unsupported_download_and_raw_module_replacement() {
    let download = run(&["resources", "files", "download", "--help"]);
    assert!(String::from_utf8_lossy(&download.stdout).contains("Always fails UNSUPPORTED"));
    let module = run(&["resources", "modules", "deploy", "--help"]);
    assert!(String::from_utf8_lossy(&module.stdout)
        .contains("Unconditionally replace raw module source"));
    let get = run(&["resources", "modules", "get", "--help"]);
    assert!(String::from_utf8_lossy(&get.stdout).contains("overwriting an existing file"));
}
