use serde_json::Value;
use std::process::{Command, Output};

fn cli() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_awtrix-cli"));
    for variable in [
        "AWTRIX_URL",
        "AWTRIX_PROFILE",
        "AWTRIX_USERNAME",
        "AWTRIX_PASSWORD",
        "AWTRIX_CONFIG",
    ] {
        command.env_remove(variable);
    }
    command.env("AWTRIX_NO_UPDATE_CHECK", "1");
    command
}

fn json_result(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn catalog_topics_support_offline_discovery_and_executable_help_links() {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("invalid.json");
    std::fs::write(&config, "invalid configuration").unwrap();
    let catalog = json_result(
        cli()
            .env("AWTRIX_CONFIG", &config)
            .env("AWTRIX_URL", "invalid-url")
            .args(["--json", "describe", "--list"])
            .output()
            .unwrap(),
    );
    for topic in catalog["topics"].as_array().unwrap() {
        let topic = topic.as_str().unwrap();
        let description = json_result(
            cli()
                .env("AWTRIX_CONFIG", &config)
                .env("AWTRIX_URL", "invalid-url")
                .args(["--json", "describe", topic, "--offline"])
                .output()
                .unwrap(),
        );
        assert_eq!(description["requested_topic"], topic);
        let help = description["syntax_help"].as_str().unwrap();
        let arguments: Vec<_> = help.split_whitespace().skip(1).collect();
        let output = cli().args(arguments).output().unwrap();
        assert!(
            output.status.success(),
            "{topic}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn profile_and_update_descriptions_ignore_connection_options_without_offline() {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("invalid.json");
    std::fs::write(&config, "invalid configuration").unwrap();
    for topic in ["profile", "profile update", "profiles", "update"] {
        let description = json_result(
            cli()
                .env("AWTRIX_CONFIG", &config)
                .env("AWTRIX_URL", "invalid-environment-url")
                .env("AWTRIX_PROFILE", "missing-profile")
                .args([
                    "--target",
                    "invalid-explicit-url",
                    "--json",
                    "describe",
                    topic,
                ])
                .output()
                .unwrap(),
        );
        assert_eq!(description["requested_topic"], topic);
        assert!(description.get("connected_capabilities").is_none());
        assert!(description.get("target").is_none());
    }
}

#[test]
fn settings_operation_metadata_matches_exact_help_flags_and_positionals() {
    for (operation, positional) in [
        ("get", None),
        ("patch", None),
        ("display-get", None),
        ("display-patch", None),
        ("brightness", Some("level")),
        ("power", Some("state")),
        ("system-get", None),
        ("reboot", None),
    ] {
        let topic = format!("settings {operation}");
        let description = json_result(
            cli()
                .args(["--json", "describe", &topic, "--offline"])
                .output()
                .unwrap(),
        );
        let help = cli()
            .args(["settings", operation, "--help"])
            .output()
            .unwrap();
        assert!(help.status.success());
        let help = String::from_utf8(help.stdout).unwrap();
        let help_flags: std::collections::BTreeSet<_> = help
            .lines()
            .filter_map(|line| line.split_whitespace().find(|word| word.starts_with("--")))
            .filter(|flag| *flag != "--help")
            .collect();
        let parameters = description["parameters"].as_object().unwrap();
        let advertised_flags: std::collections::BTreeSet<_> = parameters
            .keys()
            .map(String::as_str)
            .filter(|key| key.starts_with("--"))
            .collect();
        assert_eq!(advertised_flags, help_flags, "{topic}");
        let positionals: Vec<_> = parameters
            .keys()
            .map(String::as_str)
            .filter(|key| !key.starts_with("--"))
            .collect();
        assert_eq!(
            positionals,
            positional.into_iter().collect::<Vec<_>>(),
            "{topic}"
        );
        if let Some(positional) = positional {
            assert!(
                help.contains(&format!("<{}>", positional.to_uppercase())),
                "{topic}"
            );
        }
    }
}

#[test]
fn unsupported_file_download_precedes_target_and_profile_resolution_without_io() {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("broken.json");
    std::fs::write(&config, "broken configuration").unwrap();
    let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    for (target, broken_config) in [
        (None, false),
        (Some("invalid-url"), false),
        (None, true),
        (Some(url.as_str()), true),
    ] {
        for existing in [false, true] {
            let output_path = directory.path().join(if existing {
                "existing.gif"
            } else {
                "absent.gif"
            });
            if existing {
                std::fs::write(&output_path, b"sentinel").unwrap();
            }
            let mut command = cli();
            command.env(
                "AWTRIX_CONFIG",
                if broken_config {
                    config.clone()
                } else {
                    directory.path().join("missing.json")
                },
            );
            if broken_config {
                command.args(["--profile", "missing-profile"]);
            }
            if let Some(target) = target {
                command.args(["--target", target]);
            }
            let result = command
                .args([
                    "--json",
                    "--fields",
                    "missing",
                    "resources",
                    "files",
                    "download",
                    "/ICONS/icon.gif",
                    "--output",
                ])
                .arg(&output_path)
                .output()
                .unwrap();
            assert_eq!(result.status.code(), Some(1), "{result:?}");
            let result: Value = serde_json::from_slice(&result.stdout).unwrap();
            assert_eq!(result["error"]["code"], "UNSUPPORTED");
            if existing {
                assert_eq!(std::fs::read(&output_path).unwrap(), b"sentinel");
            } else {
                assert!(!output_path.exists());
            }
        }
    }
    assert!(server
        .recv_timeout(std::time::Duration::from_millis(100))
        .unwrap()
        .is_none());
}

#[test]
fn unknown_projection_field_does_not_undo_local_minify_output() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("demo.ax");
    std::fs::write(&source, "# remove comment\nvar value = 1\nprint(value)\n").unwrap();
    let output = cli()
        .args(["--json", "--fields", "missing", "minify"])
        .arg(&source)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let error: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(error["error"]["code"], "UNKNOWN_FIELD");
    let minified = std::fs::read(directory.path().join("demo.min.ax")).unwrap();
    assert!(!minified.is_empty());
    assert!(!String::from_utf8(minified)
        .unwrap()
        .contains("remove comment"));
}
