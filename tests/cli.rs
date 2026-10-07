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
fn project_init_creates_discoverable_manifest_and_valid_berry_entrypoint() {
    let root = std::env::temp_dir().join(format!("awtrix-project-init-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let output = run(&["--json", "project", "init", root.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(root.join("awtrix.toml").exists());
    let source = std::fs::read_to_string(root.join("src/main.be")).unwrap();
    assert!(source.contains("# @name main"));
    assert!(source.contains("def loop()"));
    assert!(source.contains("def draw()"));
    assert!(source.contains("return ProjectApp()"));
    let description = run(&["--json", "describe", "project"]);
    assert!(description.status.success());
    assert!(String::from_utf8_lossy(&description.stdout).contains("awtrix project init"));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn checked_in_project_example_preflights_its_gif_module_and_script() {
    let output = run(&[
        "--json",
        "project",
        "validate",
        "--manifest",
        "examples/project/awtrix.toml",
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["valid"], true);
    assert_eq!(result["modules"], 1);
    assert_eq!(result["resources"], 1);
    assert_eq!(result["scripts"], 1);
}

#[test]
fn project_validation_checks_all_local_files_before_any_remote_request() {
    let root = std::env::temp_dir().join(format!("awtrix-project-invalid-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("awtrix.toml"), "[project]\nname='broken'\nversion='1'\n[[scripts]]\nname='main'\nfile='missing.be'\ncreate=true\n").unwrap();
    let output = run(&[
        "--target",
        "http://127.0.0.1:1",
        "--json",
        "project",
        "deploy",
        "--manifest",
        root.join("awtrix.toml").to_str().unwrap(),
    ]);
    assert_eq!(output.status.code(), Some(1));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["error"]["code"], "PROJECT_INVALID");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn project_preflight_rejects_conflicting_create_config_targets_icon_magic_and_module_tokens() {
    let root =
        std::env::temp_dir().join(format!("awtrix-project-contracts-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("main.be"), "print(1)\n").unwrap();
    std::fs::write(root.join("expected.be"), "before\n").unwrap();
    std::fs::write(root.join("one.json"), "{}\n").unwrap();
    std::fs::write(root.join("two.json"), "{}\n").unwrap();
    std::fs::write(root.join("icon.gif"), b"not a GIF").unwrap();
    std::fs::write(root.join("bad-module.be"), "# @modulex\n").unwrap();
    let cases = [
        ("create-reference", "[project]\nname='x'\nversion='1'\n[[scripts]]\nname='main'\nfile='main.be'\ncreate=true\nexpected_source_file='expected.be'\n", "PROJECT_INVALID"),
        ("duplicate-config", "[project]\nname='x'\nversion='1'\n[[scripts]]\nname='main'\nfile='main.be'\ncreate=true\n[[config]]\nscript='main'\nfile='one.json'\n[[config]]\nscript='main'\nfile='two.json'\n", "PROJECT_INVALID"),
        ("icon-magic", "[project]\nname='x'\nversion='1'\n[[resources]]\npath='/ICONS/bad.gif'\nfile='icon.gif'\n", "INVALID_RESOURCE"),
        ("module-token", "[project]\nname='x'\nversion='1'\n[[modules]]\nname='broken'\nfile='bad-module.be'\n", "PROJECT_INVALID"),
    ];
    for (name, manifest, expected_code) in cases {
        let manifest_path = root.join(format!("{name}.toml"));
        std::fs::write(&manifest_path, manifest).unwrap();
        let output = run(&[
            "--target",
            "http://127.0.0.1:1",
            "--json",
            "project",
            "deploy",
            "--manifest",
            manifest_path.to_str().unwrap(),
        ]);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{name}: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["error"]["code"], expected_code, "{name}");
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn project_deployment_orders_dependencies_and_reports_partial_failure_additively() {
    let root = std::env::temp_dir().join(format!("awtrix-project-deploy-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("src/helpers.be"),
        "# @module helpers\ndef value()\n return 1\nend\n",
    )
    .unwrap();
    std::fs::write(
        root.join("src/main.be"),
        "# @name main\ndef loop()\n return true\nend\n",
    )
    .unwrap();
    std::fs::write(root.join("src/expected.be"), "old source\n").unwrap();
    std::fs::write(root.join("icon.bin"), b"asset").unwrap();
    std::fs::write(root.join("awtrix.toml"), "[project]\nname='sample'\nversion='1'\n[[modules]]\nname='helpers'\nfile='src/helpers.be'\n[[resources]]\npath='/FILES/icon.bin'\nfile='icon.bin'\n[[scripts]]\nname='main'\nfile='src/main.be'\nexpected_source_file='src/expected.be'\n").unwrap();
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        let mut routes = Vec::new();
        for _ in 0..6 {
            let mut r = server.recv().unwrap();
            routes.push((r.method().to_string(), r.url().to_owned()));
            match routes.last().unwrap().1.as_str() {
                "/api/v1/apps/script/helpers" => {
                    let _ = r.respond(Response::from_string("{}"));
                }
                "/api/v1/files?dir=/FILES" => {
                    let _ = r.respond(Response::from_string("{}"));
                }
                "/api/v1/capabilities" => {
                    let _ = r.respond(Response::from_string("{\"scriptUpdates\":true}"));
                }
                "/api/v1/apps/script-update/main" => {
                    let mut body = String::new();
                    r.as_reader().read_to_string(&mut body).unwrap();
                    assert_eq!(
                        serde_json::from_str::<serde_json::Value>(&body).unwrap()
                            ["expected_source"],
                        "old source\n"
                    );
                    let _ = r.respond(Response::from_string("{}"));
                }
                "/api/v1/system" => {
                    let _ = r.respond(Response::from_string("{\"scriptingEnabled\":true}"));
                }
                "/api/v1/apps" => {
                    let _=r.respond(Response::from_string("[{\"name\":\"main\",\"origin\":\"script\",\"enabled\":true,\"error\":null}]"));
                }
                other => panic!("unexpected route {other}"),
            }
        }
        routes
    });
    let out = run(&[
        "--target",
        &url,
        "--json",
        "project",
        "deploy",
        "--manifest",
        root.join("awtrix.toml").to_str().unwrap(),
    ]);
    let routes = worker.join().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert_eq!(
        routes.iter().map(|(_, u)| u.as_str()).collect::<Vec<_>>(),
        vec![
            "/api/v1/apps/script/helpers",
            "/api/v1/files?dir=/FILES",
            "/api/v1/capabilities",
            "/api/v1/apps/script-update/main",
            "/api/v1/system",
            "/api/v1/apps"
        ]
    );
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["additive"], true);
    assert_eq!(report["target_origin"], "command-line");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn project_stops_on_first_remote_error_and_reports_unrun_work_without_deleting_foreign_items() {
    let root = std::env::temp_dir().join(format!("awtrix-project-partial-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("module.be"), "# @module helpers\n").unwrap();
    std::fs::write(root.join("main.be"), "print(1)\n").unwrap();
    std::fs::write(root.join("asset"), b"asset").unwrap();
    std::fs::write(root.join("awtrix.toml"),"[project]\nname='partial'\nversion='1'\n[[modules]]\nname='helpers'\nfile='module.be'\n[[resources]]\npath='/FILES/a.bin'\nfile='asset'\n[[scripts]]\nname='main'\nfile='main.be'\ncreate=true\n").unwrap();
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        let r = server.recv().unwrap();
        assert_eq!(r.url(), "/api/v1/apps/script/helpers");
        r.respond(Response::from_string("{}").with_status_code(500))
            .unwrap();
    });
    let out = run(&[
        "--target",
        &url,
        "--json",
        "project",
        "deploy",
        "--manifest",
        root.join("awtrix.toml").to_str().unwrap(),
    ]);
    worker.join().unwrap();
    assert_eq!(out.status.code(), Some(1));
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(report["error"]["code"], "PROJECT_DEPLOY_FAILED");
    assert_eq!(report["report"]["succeeded"], serde_json::json!([]));
    assert_eq!(report["report"]["failed_operation"], "module:helpers");
    assert_eq!(
        report["report"]["not_run"],
        serde_json::json!(["resource:/FILES/a.bin", "script:main"])
    );
    assert_eq!(report["report"]["transactional"], false);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn project_profile_is_used_below_explicit_and_environment_targets() {
    let root = std::env::temp_dir().join(format!("awtrix-project-targets-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let config = root.join("config.json");
    let project_server = Server::http("127.0.0.1:0").unwrap();
    let project_url = format!("http://{}", project_server.server_addr());
    assert!(run_with_config(
        &["profile", "add", "project-device", "--target", &project_url],
        &config
    )
    .status
    .success());
    std::fs::write(root.join("module.be"), "# @module helper\n").unwrap();
    std::fs::write(root.join("awtrix.toml"), "[project]\nname='targets'\nversion='1'\n[target]\nprofile='project-device'\n[[modules]]\nname='helper'\nfile='module.be'\n").unwrap();
    let project_worker = thread::spawn(move || {
        let r = project_server.recv().unwrap();
        assert_eq!(r.url(), "/api/v1/apps/script/helper");
        r.respond(Response::from_string("{}")).unwrap();
    });
    let project = Command::new(env!("CARGO_BIN_EXE_awtrix"))
        .args([
            "--json",
            "project",
            "deploy",
            "--manifest",
            root.join("awtrix.toml").to_str().unwrap(),
        ])
        .env("AWTRIX_CONFIG", &config)
        .env_remove("AWTRIX_URL")
        .output()
        .unwrap();
    project_worker.join().unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&project.stdout).unwrap()["target_origin"],
        "project-profile"
    );
    let env_server = Server::http("127.0.0.1:0").unwrap();
    let env_url = format!("http://{}", env_server.server_addr());
    let env_worker = thread::spawn(move || {
        let r = env_server.recv().unwrap();
        r.respond(Response::from_string("{}")).unwrap();
    });
    let env_out = Command::new(env!("CARGO_BIN_EXE_awtrix"))
        .args([
            "--json",
            "project",
            "deploy",
            "--manifest",
            root.join("awtrix.toml").to_str().unwrap(),
        ])
        .env("AWTRIX_CONFIG", &config)
        .env("AWTRIX_URL", &env_url)
        .output()
        .unwrap();
    env_worker.join().unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&env_out.stdout).unwrap()["target_origin"],
        "environment"
    );
    let explicit_server = Server::http("127.0.0.1:0").unwrap();
    let explicit_url = format!("http://{}", explicit_server.server_addr());
    let explicit_worker = thread::spawn(move || {
        let r = explicit_server.recv().unwrap();
        r.respond(Response::from_string("{}")).unwrap();
    });
    let explicit = Command::new(env!("CARGO_BIN_EXE_awtrix"))
        .args([
            "--json",
            "--target",
            &explicit_url,
            "project",
            "deploy",
            "--manifest",
            root.join("awtrix.toml").to_str().unwrap(),
        ])
        .env("AWTRIX_CONFIG", &config)
        .env("AWTRIX_URL", &env_url)
        .output()
        .unwrap();
    explicit_worker.join().unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&explicit.stdout).unwrap()["target_origin"],
        "command-line"
    );
    let _ = std::fs::remove_dir_all(root);
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
fn pushed_apps_create_update_and_delete_use_documented_routes_and_payload() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        for method in ["PUT", "PUT", "DELETE"] {
            let mut request = server.recv().unwrap();
            assert_eq!(request.method().as_str(), method);
            assert!(request.url().ends_with(if method == "DELETE" {
                "/api/v1/apps/build"
            } else {
                "/api/v1/apps/pushed/build"
            }));
            if method == "PUT" {
                let mut body = String::new();
                request.as_reader().read_to_string(&mut body).unwrap();
                let payload: serde_json::Value = serde_json::from_str(&body).unwrap();
                assert_eq!(payload["text"], "working");
                assert_eq!(payload["lifetimeMs"], 2500);
            }
            request.respond(Response::from_string("{}")).unwrap();
        }
    });
    for operation in ["create", "update"] {
        let output = run(&[
            "--target",
            &url,
            "--json",
            "apps",
            operation,
            "build",
            "--payload",
            r#"{"text":"working","lifetimeMs":2500}"#,
        ]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
    let output = run(&["--target", &url, "--json", "apps", "delete", "build"]);
    assert!(output.status.success());
    worker.join().unwrap();
}

#[test]
fn pushed_app_invalid_payloads_are_rejected_before_http() {
    for payload in [
        r#"{"text":7}"#,
        r#"{"notAField":1}"#,
        r#"{"text":"x","lifetimeExpiry":"later"}"#,
    ] {
        let output = run(&[
            "--target",
            "http://127.0.0.1:1",
            "--json",
            "apps",
            "create",
            "build",
            "--payload",
            payload,
        ]);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["error"]["code"],
            "ARGUMENT"
        );
    }
}

#[test]
fn pushed_app_description_documents_json_fields_and_expiration() {
    for operation in ["create", "update", "delete"] {
        let topic = format!("apps {operation}");
        let output = run(&["--json", "describe", &topic]);
        assert!(output.status.success());
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["command"], topic);
        assert!(!value["schemas"]["output"].is_null());
        assert!(value["examples"]
            .as_array()
            .is_some_and(|items| !items.is_empty()));
        if operation != "delete" {
            assert!(value["outputs"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v.as_str().unwrap().contains("visibility")));
            assert!(value["parameters"]["--payload"]
                .as_str()
                .unwrap()
                .contains("lifetimeMs"));
            assert!(value["parameters"]["--file"].as_str().is_some());
            let help = run(&["apps", operation, "--help"]);
            assert!(help.status.success());
            assert!(String::from_utf8_lossy(&help.stdout).contains("upsert"));
        }
    }
}

#[test]
fn pushed_app_remote_capacity_rejection_is_reported_as_http_failure() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        let request = server.recv().unwrap();
        assert_eq!(request.url(), "/api/v1/apps/pushed/build");
        request
            .respond(Response::from_string("private upstream detail").with_status_code(507))
            .unwrap();
    });
    let output = run(&[
        "--target",
        &url,
        "--json",
        "apps",
        "create",
        "build",
        "--payload",
        r#"{"text":"working"}"#,
    ]);
    worker.join().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["error"]["code"],
        "HTTP_507"
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains("private upstream detail"));
}

#[test]
fn pushed_app_large_payload_uses_variant_specific_official_http_limit() {
    for (variant, board, soc, should_write) in [
        ("ESP32", "awtrixng", "esp32", false),
        ("ESP32-S3", "awtrixng", "esp32s3", false),
        ("TC002", "tc002", "esp32s3", true),
    ] {
        let server = Server::http("127.0.0.1:0").unwrap();
        let url = format!("http://{}", server.server_addr());
        let worker = thread::spawn(move || {
            let identity = server
                .recv_timeout(Duration::from_secs(3))
                .unwrap()
                .unwrap();
            assert_eq!(identity.url(), "/api/v1/device");
            identity
                .respond(Response::from_string(format!(
                    r#"{{"boardType":"{board}","soc":"{soc}"}}"#
                )))
                .unwrap();
            if should_write {
                let mut mutation = server
                    .recv_timeout(Duration::from_secs(3))
                    .unwrap()
                    .unwrap();
                assert_eq!(mutation.url(), "/api/v1/apps/pushed/large");
                let mut body = String::new();
                mutation.as_reader().read_to_string(&mut body).unwrap();
                assert!(body.len() > 8192);
                mutation.respond(Response::from_string("{}")).unwrap();
            }
        });
        let path = std::env::temp_dir().join(format!(
            "awtrix-app-large-{}-{variant}.json",
            std::process::id()
        ));
        let large_payload = format!(r#"{{"text":"{}"}}"#, "x".repeat(9000));
        std::fs::write(&path, large_payload).unwrap();
        let output = run(&[
            "--target",
            &url,
            "--json",
            "apps",
            "create",
            "large",
            "--file",
            path.to_str().unwrap(),
        ]);
        let _ = std::fs::remove_file(path);
        worker.join().unwrap();
        assert_eq!(
            output.status.success(),
            should_write,
            "{variant}: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        if !should_write {
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["error"]
                    ["code"],
                "ARGUMENT"
            );
        }
    }
}

#[test]
fn pushed_app_body_above_largest_variant_limit_is_rejected_before_http() {
    let path =
        std::env::temp_dir().join(format!("awtrix-app-too-large-{}.json", std::process::id()));
    let payload = format!(r#"{{"text":"{}"}}"#, "x".repeat(2 * 1024 * 1024));
    std::fs::write(&path, payload).unwrap();
    let output = run(&[
        "--target",
        "http://127.0.0.1:1",
        "--json",
        "apps",
        "create",
        "large",
        "--file",
        path.to_str().unwrap(),
    ]);
    let _ = std::fs::remove_file(path);
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["error"]["code"],
        "ARGUMENT"
    );
}

#[test]
fn pushed_app_large_payload_on_unknown_variant_fails_closed_after_identity() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        let identity = server
            .recv_timeout(Duration::from_secs(3))
            .unwrap()
            .unwrap();
        assert_eq!(identity.url(), "/api/v1/device");
        identity
            .respond(Response::from_string(
                r#"{"boardType":"future-board","soc":"unknown"}"#,
            ))
            .unwrap();
    });
    let payload = format!(r#"{{"text":"{}"}}"#, "x".repeat(9000));
    let output = run(&[
        "--target",
        &url,
        "--json",
        "apps",
        "create",
        "large",
        "--payload",
        &payload,
    ]);
    worker.join().unwrap();
    assert_eq!(output.status.code(), Some(6));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["error"]["code"],
        "INCOMPATIBLE"
    );
}

#[test]
fn headless_description_documents_lifecycle_and_hardware_limits() {
    let output = run(&["--json", "describe", "headless"]);
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(value["examples"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v.as_str().unwrap().contains("headless start")));
    assert!(value["prerequisites"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v.as_str().unwrap().contains("sensors")));
}

#[test]
fn headless_start_requires_a_user_supplied_binary() {
    let output = run(&["--json", "headless", "start"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["error"]["code"],
        "BINARY_REQUIRED"
    );
}

#[test]
fn headless_start_reports_missing_binary_without_creating_state() {
    let root = std::env::temp_dir().join(format!("awtrix-headless-missing-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let config = root.join("config.json");
    let output = Command::new(env!("CARGO_BIN_EXE_awtrix"))
        .args([
            "--json",
            "headless",
            "start",
            "--binary",
            root.join("not-awtrix").to_str().unwrap(),
        ])
        .env("AWTRIX_CONFIG", &config)
        .output()
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["error"]["code"],
        "BINARY_NOT_FOUND"
    );
    assert!(!root.join("headless.json").exists());
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(target_os = "linux")]
#[test]
fn headless_refuses_occupied_port_before_spawning_child_or_claiming_external_service() {
    use std::os::unix::fs::PermissionsExt;
    let root =
        std::env::temp_dir().join(format!("awtrix-headless-occupied-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let server = Server::http("127.0.0.1:0").unwrap();
    let port = server.server_addr().to_ip().unwrap().port().to_string();
    let config = root.join("config.json");
    let marker = root.join("spawned");
    let fake = root.join("sleeping-child");
    std::fs::write(
        &fake,
        format!("#!/bin/sh\ntouch '{}'\nexec sleep 30\n", marker.display()),
    )
    .unwrap();
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o700)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_awtrix"))
        .args([
            "--json",
            "headless",
            "start",
            "--binary",
            fake.to_str().unwrap(),
            "--port",
            &port,
        ])
        .env("AWTRIX_CONFIG", &config)
        .output()
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["error"]["code"],
        "PORT_IN_USE"
    );
    assert!(
        !marker.exists(),
        "occupied-port rejection must happen before child spawn"
    );
    assert!(!root.join("headless.json").exists());
    let stop = Command::new(env!("CARGO_BIN_EXE_awtrix"))
        .args(["--json", "headless", "stop"])
        .env("AWTRIX_CONFIG", &config)
        .output()
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&stop.stdout).unwrap()["error"]["code"],
        "NOT_RUNNING"
    );
    drop(server);
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(target_os = "linux")]
#[test]
fn headless_never_claims_or_stops_an_external_http_listener_winning_spawn_race() {
    use std::{
        os::unix::fs::PermissionsExt,
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
    };
    let root =
        std::env::temp_dir().join(format!("awtrix-headless-port-race-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let reservation = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = reservation.local_addr().unwrap().port();
    drop(reservation);
    let spawned = root.join("spawned");
    let release = root.join("release");
    let child_pid_file = root.join("child.pid");
    let fake = root.join("delayed-child");
    std::fs::write(&fake,format!("#!/bin/sh\necho $$ > '{}'\ntouch '{}'\nwhile [ ! -e '{}' ]; do sleep 0.02; done\nexec sleep 30\n",child_pid_file.display(),spawned.display(),release.display())).unwrap();
    std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o700)).unwrap();
    let config = root.join("config.json");
    let cli = Command::new(env!("CARGO_BIN_EXE_awtrix"))
        .args([
            "--json",
            "headless",
            "start",
            "--binary",
            fake.to_str().unwrap(),
            "--port",
            &port.to_string(),
            "--ready-timeout-secs",
            "1",
        ])
        .env("AWTRIX_CONFIG", &config)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    for _ in 0..100 {
        if spawned.exists() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    let server = Server::http(format!("127.0.0.1:{port}"))
        .expect("port reservation is released immediately before spawn");
    let serving = Arc::new(AtomicBool::new(true));
    let worker_flag = serving.clone();
    let worker = thread::spawn(move || {
        while worker_flag.load(Ordering::SeqCst) {
            if let Ok(Some(request)) = server.recv_timeout(Duration::from_millis(50)) {
                let _ = request.respond(Response::from_string("{\"external\":true}"));
            }
        }
    });
    std::fs::write(&release, b"go").unwrap();
    let cli_pid = cli.id();
    let output = cli.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(4));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["error"]["code"],
        "TIMEOUT"
    );
    let child_pid: u32 = std::fs::read_to_string(&child_pid_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert!(!std::path::Path::new(&format!("/proc/{child_pid}")).exists());
    assert!(!root.join("headless.json").exists());
    assert!(!std::fs::read_dir(std::env::temp_dir())
        .unwrap()
        .flatten()
        .any(|entry| entry
            .file_name()
            .to_string_lossy()
            .starts_with(&format!("awtrix-headless-{cli_pid}-"))));
    assert!(
        reqwest::blocking::get(format!("http://127.0.0.1:{port}/api/v1/device"))
            .unwrap()
            .status()
            .is_success(),
        "the pre-existing external listener must remain alive"
    );
    serving.store(false, Ordering::SeqCst);
    worker.join().unwrap();
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(target_os = "linux")]
#[test]
fn headless_fake_process_start_status_stop_and_isolated_cleanup() {
    use std::os::unix::fs::PermissionsExt;
    let root = std::env::temp_dir().join(format!(
        "awtrix-headless-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let config = root.join("config.json");
    let binary = root.join("fake-awtrix");
    std::fs::write(&binary, "#!/usr/bin/env python3\nimport argparse,json\nfrom http.server import BaseHTTPRequestHandler,HTTPServer\np=argparse.ArgumentParser();p.add_argument('--data');p.add_argument('--port',type=int);p.add_argument('--width');p.add_argument('--height');a=p.parse_args()\nclass H(BaseHTTPRequestHandler):\n def do_GET(self):\n  b=json.dumps({'boardType':'awtrixng','soc':'esp32','width':52,'height':16}).encode();self.send_response(200);self.send_header('Content-Length',str(len(b)));self.end_headers();self.wfile.write(b)\n def log_message(self,*args): pass\nHTTPServer(('127.0.0.1',a.port),H).serve_forever()\n").unwrap();
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o700)).unwrap();
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
        .to_string();
    let output = Command::new(env!("CARGO_BIN_EXE_awtrix"))
        .args([
            "--json",
            "headless",
            "start",
            "--binary",
            binary.to_str().unwrap(),
            "--port",
            &port,
        ])
        .env("AWTRIX_CONFIG", &config)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let started: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(started["owned"], true);
    let data = std::path::PathBuf::from(started["data"].as_str().unwrap());
    assert!(data.exists());
    let status = Command::new(env!("CARGO_BIN_EXE_awtrix"))
        .args(["--json", "headless", "status"])
        .env("AWTRIX_CONFIG", &config)
        .output()
        .unwrap();
    assert!(status.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&status.stdout).unwrap()["running"],
        true
    );
    let stopped = Command::new(env!("CARGO_BIN_EXE_awtrix"))
        .args(["--json", "headless", "stop"])
        .env("AWTRIX_CONFIG", &config)
        .output()
        .unwrap();
    assert!(
        stopped.status.success(),
        "{}",
        String::from_utf8_lossy(&stopped.stdout)
    );
    assert!(!data.exists());
    assert!(!root.join("headless.json").exists());
    let persistent = root.join("persistent");
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
        .to_string();
    let persistent_start = Command::new(env!("CARGO_BIN_EXE_awtrix"))
        .args([
            "--json",
            "headless",
            "start",
            "--binary",
            binary.to_str().unwrap(),
            "--port",
            &port,
            "--data",
            persistent.to_str().unwrap(),
        ])
        .env("AWTRIX_CONFIG", &config)
        .output()
        .unwrap();
    assert!(
        persistent_start.status.success(),
        "{}",
        String::from_utf8_lossy(&persistent_start.stdout)
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&persistent_start.stdout).unwrap()
            ["temporary_data"],
        false
    );
    let persistent_stop = Command::new(env!("CARGO_BIN_EXE_awtrix"))
        .args(["--json", "headless", "stop"])
        .env("AWTRIX_CONFIG", &config)
        .output()
        .unwrap();
    assert!(persistent_stop.status.success());
    assert!(
        persistent.exists(),
        "explicitly configured persistent data must survive stop"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(target_os = "linux")]
#[test]
fn headless_start_timeout_reaps_its_child_and_preserves_external_processes() {
    use std::os::unix::fs::PermissionsExt;
    let root = std::env::temp_dir().join(format!("awtrix-headless-timeout-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let config = root.join("config.json");
    let binary = root.join("no-http");
    let child_pid_file = root.join("child.pid");
    std::fs::write(
        &binary,
        format!(
            "#!/bin/sh\necho $$ > '{}'\nexec sleep 30\n",
            child_pid_file.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o700)).unwrap();
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
        .to_string();
    let output = Command::new(env!("CARGO_BIN_EXE_awtrix"))
        .args([
            "--json",
            "headless",
            "start",
            "--binary",
            binary.to_str().unwrap(),
            "--port",
            &port,
            "--ready-timeout-secs",
            "1",
        ])
        .env("AWTRIX_CONFIG", &config)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(4));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["error"]["code"],
        "TIMEOUT"
    );
    assert!(!root.join("headless.json").exists());
    let child_pid: u32 = std::fs::read_to_string(&child_pid_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert!(
        !std::path::Path::new(&format!("/proc/{child_pid}")).exists(),
        "timed out child must be reaped/stopped"
    );
    let external = std::process::Command::new("sleep")
        .arg("30")
        .spawn()
        .unwrap();
    let stop = Command::new(env!("CARGO_BIN_EXE_awtrix"))
        .args(["--json", "headless", "stop"])
        .env("AWTRIX_CONFIG", &config)
        .output()
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&stop.stdout).unwrap()["error"]["code"],
        "NOT_RUNNING"
    );
    assert!(std::path::Path::new(&format!("/proc/{}", external.id())).exists());
    let mut external = external;
    external.kill().unwrap();
    let _ = external.wait();
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(target_os = "linux")]
#[test]
fn headless_ctrl_c_during_readiness_stops_child_and_cleans_isolated_data() {
    use std::os::unix::fs::PermissionsExt;
    let root =
        std::env::temp_dir().join(format!("awtrix-headless-interrupt-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let config = root.join("config.json");
    let binary = root.join("no-http");
    let child_pid_file = root.join("child.pid");
    std::fs::write(
        &binary,
        format!(
            "#!/bin/sh\necho $$ > '{}'\nexec sleep 30\n",
            child_pid_file.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o700)).unwrap();
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
        .to_string();
    let cli = Command::new(env!("CARGO_BIN_EXE_awtrix"))
        .args([
            "--json",
            "headless",
            "start",
            "--binary",
            binary.to_str().unwrap(),
            "--port",
            &port,
            "--ready-timeout-secs",
            "30",
        ])
        .env("AWTRIX_CONFIG", &config)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    for _ in 0..100 {
        if child_pid_file.exists() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    thread::sleep(Duration::from_millis(200));
    let signal = Command::new("kill")
        .args(["-INT", &cli.id().to_string()])
        .status()
        .unwrap();
    assert!(signal.success());
    let cli_pid = cli.id();
    let output = cli.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["error"]["code"],
        "INTERRUPTED"
    );
    let child_pid: u32 = std::fs::read_to_string(&child_pid_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert!(!std::path::Path::new(&format!("/proc/{child_pid}")).exists());
    assert!(!root.join("headless.json").exists());
    assert!(!std::fs::read_dir(std::env::temp_dir())
        .unwrap()
        .flatten()
        .any(|entry| entry
            .file_name()
            .to_string_lossy()
            .starts_with(&format!("awtrix-headless-{cli_pid}-"))));
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(target_os = "linux")]
#[test]
fn headless_stop_rejects_a_stale_pid_identity_without_signaling_it() {
    let root =
        std::env::temp_dir().join(format!("awtrix-headless-pid-reuse-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let config = root.join("config.json");
    let stat = std::fs::read_to_string(format!("/proc/{}/stat", std::process::id())).unwrap();
    let start: u64 = stat
        .rsplit_once(") ")
        .unwrap()
        .1
        .split_whitespace()
        .nth(19)
        .unwrap()
        .parse()
        .unwrap();
    std::fs::write(root.join("headless.json"),serde_json::json!({"pid":std::process::id(),"start_time":start+1,"url":"http://127.0.0.1:8080","data":root,"temporary":true}).to_string()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_awtrix"))
        .args(["--json", "headless", "stop"])
        .env("AWTRIX_CONFIG", &config)
        .output()
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["error"]["code"],
        "NOT_RUNNING"
    );
    assert!(std::path::Path::new(&format!("/proc/{}", std::process::id())).exists());
    let _ = std::fs::remove_dir_all(root);
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
    assert_eq!(value["observed_window"]["complete"], false);
    assert_eq!(
        value["observed_window"]["early_termination_reason"],
        "berry_error"
    );
    assert!(value["observed_window"]["elapsed_ms"].is_number());
    assert!(String::from_utf8_lossy(&output.stderr).contains("BERRY_ERROR"));
}

#[test]
fn script_verify_absent_or_disabled_returns_nonzero_without_logs() {
    for (apps, reason) in [
        ("[]", "script_absent"),
        (
            r#"[{"name":"demo","origin":"script","enabled":false,"error":null}]"#,
            "script_disabled",
        ),
    ] {
        let server = Server::http("127.0.0.1:0").unwrap();
        let url = format!("http://{}", server.server_addr());
        let apps = apps.to_owned();
        let worker = thread::spawn(move || {
            for (route, body) in [
                ("/api/v1/system", r#"{"scriptingEnabled":true}"#.to_owned()),
                ("/api/v1/apps", apps),
            ] {
                let request = server.recv().unwrap();
                assert_eq!(request.url(), route);
                request.respond(Response::from_string(body)).unwrap();
            }
        });
        let output = run(&["--target", &url, "--json", "script", "verify", "demo"]);
        worker.join().unwrap();
        assert_eq!(output.status.code(), Some(1));
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["not_available"], reason);
        assert_eq!(report["start_verified"], false);
    }
}

#[test]
fn script_verify_global_disable_and_initial_http_errors_return_reports() {
    let disabled = Server::http("127.0.0.1:0").unwrap();
    let disabled_url = format!("http://{}", disabled.server_addr());
    let worker = thread::spawn(move || {
        disabled
            .recv()
            .unwrap()
            .respond(Response::from_string(r#"{"scriptingEnabled":false}"#))
            .unwrap()
    });
    let output = run(&[
        "--target",
        &disabled_url,
        "--json",
        "script",
        "verify",
        "demo",
    ]);
    worker.join().unwrap();
    assert_eq!(output.status.code(), Some(1));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["not_available"], "scripting_disabled");
    assert_eq!(report["observed_window"]["complete"], false);

    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        server
            .recv()
            .unwrap()
            .respond(Response::from_string(r#"{"scriptingEnabled":true}"#))
            .unwrap();
        server.recv().unwrap().respond(Response::from_string(r#"[{"name":"demo","origin":"script","enabled":true,"error":{"message":"compile failure","line":3}}]"#)).unwrap();
    });
    let output = run(&["--target", &url, "--json", "script", "verify", "demo"]);
    worker.join().unwrap();
    assert_eq!(output.status.code(), Some(1));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["runtime_error"]["line"], 3);
    assert_eq!(
        report["observed_window"]["early_termination_reason"],
        "berry_error"
    );

    for (route, phase) in [("/api/v1/system", "system"), ("/api/v1/apps", "apps")] {
        let server = Server::http("127.0.0.1:0").unwrap();
        let url = format!("http://{}", server.server_addr());
        let route = route.to_owned();
        let phase = phase.to_owned();
        let worker = thread::spawn(move || {
            if route == "/api/v1/apps" {
                server
                    .recv()
                    .unwrap()
                    .respond(Response::from_string(r#"{"scriptingEnabled":true}"#))
                    .unwrap();
            }
            server
                .recv()
                .unwrap()
                .respond(Response::from_string("private").with_status_code(500))
                .unwrap();
        });
        let output = run(&["--target", &url, "--json", "script", "verify", "demo"]);
        worker.join().unwrap();
        assert_eq!(output.status.code(), Some(1));
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["not_available"]["code"], "HTTP");
        assert_eq!(report["not_available"]["phase"], phase);
        assert!(!String::from_utf8_lossy(&output.stdout).contains("private"));
    }
}

#[test]
fn script_verify_initial_system_and_apps_timeouts_are_bounded_reports() {
    for (slow_route, phase) in [("/api/v1/system", "system"), ("/api/v1/apps", "apps")] {
        let server = Server::http("127.0.0.1:0").unwrap();
        let url = format!("http://{}", server.server_addr());
        let slow_route = slow_route.to_owned();
        let worker = thread::spawn(move || {
            if slow_route == "/api/v1/apps" {
                server
                    .recv()
                    .unwrap()
                    .respond(Response::from_string(r#"{"scriptingEnabled":true}"#))
                    .unwrap();
            }
            let request = server.recv().unwrap();
            thread::sleep(Duration::from_millis(1300));
            let _ = request.respond(Response::from_string(if slow_route == "/api/v1/system" {
                r#"{"scriptingEnabled":true}"#
            } else {
                "[]"
            }));
        });
        let start = std::time::Instant::now();
        let output = run(&[
            "--target",
            &url,
            "--timeout",
            "3000",
            "--json",
            "script",
            "verify",
            "demo",
            "--duration-secs",
            "1",
        ]);
        let elapsed = start.elapsed();
        worker.join().unwrap();
        assert!(
            elapsed < Duration::from_millis(1250),
            "request exceeded verify deadline: {elapsed:?}"
        );
        assert_eq!(output.status.code(), Some(1));
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["not_available"]["code"], "TIMEOUT");
        assert_eq!(report["not_available"]["phase"], phase);
    }
}

#[test]
fn script_verify_reports_script_disappearance_or_disable_during_window() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        server
            .recv()
            .unwrap()
            .respond(Response::from_string(r#"{"scriptingEnabled":true}"#))
            .unwrap();
        server
            .recv()
            .unwrap()
            .respond(Response::from_string(
                r#"[{"name":"demo","origin":"script","enabled":true,"error":null}]"#,
            ))
            .unwrap();
        server
            .recv()
            .unwrap()
            .respond(Response::from_string(r#"{"next":2,"lines":[]}"#))
            .unwrap();
        server
            .recv()
            .unwrap()
            .respond(Response::from_string("[]"))
            .unwrap();
    });
    let output = run(&[
        "--target",
        &url,
        "--json",
        "script",
        "verify",
        "demo",
        "--duration-secs",
        "1",
        "--interval-ms",
        "1",
    ]);
    worker.join().unwrap();
    assert_eq!(output.status.code(), Some(1));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["not_available"]["code"], "SCRIPT_NOT_RUNNING");
    assert_eq!(report["observed_window"]["complete"], false);
    assert_eq!(
        report["observed_window"]["early_termination_reason"],
        "script_not_running"
    );
}

#[test]
fn script_verify_deadline_bounds_slow_log_request() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        server
            .recv()
            .unwrap()
            .respond(Response::from_string(r#"{"scriptingEnabled":true}"#))
            .unwrap();
        server
            .recv()
            .unwrap()
            .respond(Response::from_string(
                r#"[{"name":"demo","origin":"script","enabled":true,"error":null}]"#,
            ))
            .unwrap();
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
        "script",
        "verify",
        "demo",
        "--duration-secs",
        "1",
        "--interval-ms",
        "1",
    ]);
    let cli_elapsed = start.elapsed();
    worker.join().unwrap();
    assert!(cli_elapsed < Duration::from_millis(1300));
    assert_eq!(output.status.code(), Some(1));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["observed_window"]["complete"], false);
    assert!(report["not_available"].is_object());
}

#[test]
fn script_verify_reports_partial_state_collection_failure() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        server
            .recv()
            .unwrap()
            .respond(Response::from_string(r#"{"scriptingEnabled":true}"#))
            .unwrap();
        server
            .recv()
            .unwrap()
            .respond(Response::from_string(
                r#"[{"name":"demo","origin":"script","enabled":true,"error":null}]"#,
            ))
            .unwrap();
        server
            .recv()
            .unwrap()
            .respond(Response::from_string(r#"{"next":8,"lines":["started"]}"#))
            .unwrap();
        server
            .recv()
            .unwrap()
            .respond(Response::from_string("private response").with_status_code(500))
            .unwrap();
    });
    let output = run(&[
        "--target",
        &url,
        "--json",
        "script",
        "verify",
        "demo",
        "--duration-secs",
        "1",
        "--interval-ms",
        "1",
    ]);
    worker.join().unwrap();
    assert_eq!(output.status.code(), Some(1));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["logs"]["next"], 8);
    assert_eq!(report["not_available"]["code"], "HTTP");
    assert_eq!(report["not_available"]["phase"], "state_observation");
    assert!(!String::from_utf8_lossy(&output.stdout).contains("private response"));
}

#[test]
fn script_deploy_verify_conflict_does_not_start_verification() {
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
            .respond(Response::from_string(r#"{}"#).with_status_code(409))
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
        "--verify-secs",
        "1",
    ]);
    worker.join().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["error"]["code"],
        "CONFLICT"
    );
}

#[test]
fn script_deploy_verify_success_keeps_atomic_deploy_and_adds_observation() {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        for (route, body) in [
            ("/api/v1/capabilities", r#"{"scriptUpdates":true}"#),
            (
                "/api/v1/apps/script-update/demo",
                r#"{"ok":true,"error":null}"#,
            ),
            ("/api/v1/system", r#"{"scriptingEnabled":true}"#),
            (
                "/api/v1/apps",
                r#"[{"name":"demo","origin":"script","enabled":true,"error":null}]"#,
            ),
            ("/api/v1/system", r#"{"scriptingEnabled":true}"#),
            (
                "/api/v1/apps",
                r#"[{"name":"demo","origin":"script","enabled":true,"error":null}]"#,
            ),
            ("/api/v1/logs?after=0", r#"{"next":3,"lines":["started"]}"#),
            (
                "/api/v1/apps",
                r#"[{"name":"demo","origin":"script","enabled":true,"error":null}]"#,
            ),
        ] {
            let request = server.recv().unwrap();
            assert_eq!(request.url(), route);
            request.respond(Response::from_string(body)).unwrap();
        }
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_millis(1200) {
            if let Some(request) = server.recv_timeout(Duration::from_millis(20)).unwrap() {
                if request.url().starts_with("/api/v1/logs?") {
                    request
                        .respond(Response::from_string(r#"{"next":3,"lines":[]}"#))
                        .unwrap();
                } else {
                    request
                        .respond(Response::from_string(
                            r#"[{"name":"demo","origin":"script","enabled":true,"error":null}]"#,
                        ))
                        .unwrap();
                }
            }
        }
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
        "--verify-secs",
        "1",
    ]);
    worker.join().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["source_saved"], true);
    assert_eq!(result["verification"]["start_verified"], true);
    assert_eq!(result["verification"]["logs"]["next"], 3);
    assert_eq!(result["verification"]["observed_window"]["complete"], true);
}

#[test]
fn script_verify_captures_png_inside_reserved_deadline_window() {
    let path = std::env::temp_dir().join(format!("awtrix-verify-{}.png", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let worker = thread::spawn(move || {
        server
            .recv()
            .unwrap()
            .respond(Response::from_string(r#"{"scriptingEnabled":true}"#))
            .unwrap();
        server
            .recv()
            .unwrap()
            .respond(Response::from_string(
                r#"[{"name":"demo","origin":"script","enabled":true,"error":null}]"#,
            ))
            .unwrap();
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_millis(1200) {
            if let Some(request) = server.recv_timeout(Duration::from_millis(20)).unwrap() {
                let body = match request.url() {
                    url if url.starts_with("/api/v1/logs?") => r#"{"next":1,"lines":["ok"]}"#,
                    "/api/v1/apps" => {
                        r#"[{"name":"demo","origin":"script","enabled":true,"error":null}]"#
                    }
                    "/api/v1/display/screen" => r#"{"width":1,"height":1,"pixels":[1122867]}"#,
                    _ => panic!("unexpected route {}", request.url()),
                };
                request.respond(Response::from_string(body)).unwrap();
            }
        }
    });
    let path_arg = path.to_string_lossy().into_owned();
    let output = run(&[
        "--target",
        &url,
        "--json",
        "script",
        "verify",
        "demo",
        "--duration-secs",
        "1",
        "--interval-ms",
        "10",
        "--capture",
        &path_arg,
    ]);
    worker.join().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["capture"]["format"], "png");
    assert_eq!(result["capture"]["width"], 1);
    assert_eq!(&std::fs::read(&path).unwrap()[..8], b"\x89PNG\r\n\x1a\n");
    let _ = std::fs::remove_file(path);
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
