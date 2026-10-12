use serde_json::{json, Value};
use std::{
    collections::BTreeSet,
    process::{Command, Output},
    thread,
    time::Duration,
};
use tiny_http::{Response, Server};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_awtrix-cli"))
        .args(args)
        .env("AWTRIX_NO_UPDATE_CHECK", "1")
        .env_remove("AWTRIX_URL")
        .env_remove("AWTRIX_CONFIG")
        .output()
        .unwrap()
}

fn report(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn describe(topic: &str) -> Value {
    report(run(&["--json", "describe", topic, "--offline"]))
}

fn exchange(args: &[&str], method: &str, path: &str, response: Value) -> (Value, Value) {
    let server = Server::http("127.0.0.1:0").unwrap();
    let url = format!("http://{}", server.server_addr());
    let method = method.to_owned();
    let path = path.to_owned();
    let worker = thread::spawn(move || {
        let mut request = server
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .expect("expected mock request");
        assert_eq!(request.method().as_str(), method);
        assert_eq!(request.url(), path);
        let mut body = String::new();
        request.as_reader().read_to_string(&mut body).unwrap();
        request
            .respond(Response::from_string(response.to_string()))
            .unwrap();
        if body.is_empty() {
            Value::Null
        } else {
            serde_json::from_str(&body).unwrap()
        }
    });
    let mut argv = vec!["--json", "--target", &url];
    argv.extend_from_slice(args);
    let output = run(&argv);
    let body = worker.join().unwrap();
    (report(output), body)
}

fn keys(value: &Value) -> BTreeSet<String> {
    value.as_object().unwrap().keys().cloned().collect()
}

#[test]
fn app_output_fields_are_exact_cli_top_level_keys_for_each_operation() {
    let inventory = json!([{"name":"demo","slot":2,"origin":"pushed","inLoop":true}, {"name":"clock","slot":null}]);
    for (args, method, path, response) in [
        (
            vec!["apps", "list"],
            "GET",
            "/api/v1/apps",
            inventory.clone(),
        ),
        (
            vec!["apps", "active-get"],
            "GET",
            "/api/v1/device",
            json!({"currentApp":"demo"}),
        ),
        (vec!["apps", "order-get"], "GET", "/api/v1/apps", inventory),
        (
            vec!["apps", "select", "demo"],
            "PUT",
            "/api/v1/apps/active",
            json!({"ok":true}),
        ),
        (
            vec!["apps", "order-set"],
            "PUT",
            "/api/v1/apps/order",
            json!({"ok":true}),
        ),
        (
            vec!["apps", "create", "demo", "--payload", r#"{"text":"Ready"}"#],
            "PUT",
            "/api/v1/apps/pushed/demo",
            json!({"ok":true}),
        ),
        (
            vec!["apps", "update", "demo", "--payload", r#"{"text":"Ready"}"#],
            "PUT",
            "/api/v1/apps/pushed/demo",
            json!({"ok":true}),
        ),
        (
            vec!["apps", "delete", "demo"],
            "DELETE",
            "/api/v1/apps/demo",
            json!({"ok":true}),
        ),
    ] {
        let topic = format!("apps {}", args[1]);
        let description = describe(&topic);
        let (actual, _) = exchange(&args, method, path, response);
        let advertised: BTreeSet<String> = description["output_fields"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_owned())
            .collect();
        assert_eq!(advertised, keys(&actual), "{topic}");
        assert_eq!(
            keys(&description["schemas"]["output"]),
            keys(&actual),
            "{topic}"
        );
    }
}

#[test]
fn settings_passthrough_responses_have_no_invented_wrapper_or_get_redaction() {
    for (args, method, path) in [
        (vec!["settings", "get"], "GET", "/api/v1/settings"),
        (vec!["settings", "display-get"], "GET", "/api/v1/display"),
        (vec!["settings", "system-get"], "GET", "/api/v1/system"),
        (
            vec!["settings", "patch", "--values", r#"{"brightness":80}"#],
            "PATCH",
            "/api/v1/settings",
        ),
        (
            vec!["settings", "display-patch", "--values", r#"{"power":true}"#],
            "PATCH",
            "/api/v1/display",
        ),
        (
            vec!["settings", "brightness", "80"],
            "PATCH",
            "/api/v1/settings",
        ),
        (vec!["settings", "power", "off"], "PATCH", "/api/v1/display"),
    ] {
        let topic = format!("settings {}", args[1]);
        let description = describe(&topic);
        assert_eq!(description["output_fields"], json!([]), "{topic}");
        assert_eq!(description["schemas"]["output"]["source"], "server-defined");
        let response = json!({"serverField":17,"nested":{"password":"fixture-secret"}});
        let (actual, _) = exchange(&args, method, path, response.clone());
        let mut expected = response;
        if args[1] == "system-get" {
            expected["nested"]["password"] = json!("[REDACTED]");
        }
        assert_eq!(actual, expected, "{topic}");
    }
    let help = run(&["settings", "get", "--help"]);
    assert!(!String::from_utf8_lossy(&help.stdout).contains("redact"));
}

#[test]
fn notification_schema_defaults_match_sent_payload_and_output_fields() {
    let description = describe("notify send");
    let shared = describe("apps create")["schemas"]["payload"]["properties"].clone();
    for (key, schema) in shared.as_object().unwrap() {
        assert_eq!(
            &description["schemas"]["payload"]["properties"][key], schema,
            "{key}"
        );
    }
    for (args, method, path) in [
        (
            vec!["notify", "send", "--payload", r#"{"text":"Ready"}"#],
            "POST",
            "/api/v1/notifications",
        ),
        (
            vec!["notify", "delete-active"],
            "DELETE",
            "/api/v1/notifications/active",
        ),
        (
            vec!["notify", "delete", "demo"],
            "DELETE",
            "/api/v1/notifications/demo",
        ),
    ] {
        let description = describe(&format!("notify {}", args[1]));
        let (actual, body) = exchange(&args, method, path, json!({"ok":true}));
        let advertised: BTreeSet<String> = description["output_fields"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_owned())
            .collect();
        assert_eq!(advertised, keys(&actual));
        if args[1] == "send" {
            for key in ["hold", "stack", "wakeup"] {
                assert_eq!(
                    body[key],
                    description["schemas"]["payload"]["properties"][key]["default"]
                );
                assert_eq!(body[key], false);
            }
        }
    }
}

// Evaluate discovery's keywords plus its documented CLI-specific integer restriction.
// is_i64 is deliberately NOT standard JSON Schema integer semantics: integral floats
// such as 1.0 pass standard JSON Schema but fail the CLI's lexical/i64 contract.
fn schema_accepts(schema: &Value, value: &Value) -> bool {
    if let Some(options) = schema["oneOf"].as_array() {
        return options.iter().filter(|s| schema_accepts(s, value)).count() == 1;
    }
    let matches_type = |kind: &str| match kind {
        "string" => value.is_string(),
        "integer" => value.is_i64(),
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        "array" => value.is_array(),
        "object" => value.is_object(),
        _ => panic!("unsupported schema type {kind}"),
    };
    let correct_type = match &schema["type"] {
        Value::String(kind) => matches_type(kind),
        Value::Array(kinds) => kinds.iter().any(|k| matches_type(k.as_str().unwrap())),
        _ => true,
    };
    if !correct_type {
        return false;
    }
    if let Some(values) = schema["enum"].as_array() {
        if !values.contains(value) {
            return false;
        }
    }
    if let Some(n) = value.as_f64() {
        if schema["minimum"].as_f64().is_some_and(|min| n < min)
            || schema["maximum"].as_f64().is_some_and(|max| n > max)
        {
            return false;
        }
    }
    if let Some(items) = value.as_array() {
        if schema["minItems"]
            .as_u64()
            .is_some_and(|min| items.len() < min as usize)
            || schema["maxItems"]
                .as_u64()
                .is_some_and(|max| items.len() > max as usize)
        {
            return false;
        }
        if !schema["items"].is_null()
            && !items
                .iter()
                .all(|item| schema_accepts(&schema["items"], item))
        {
            return false;
        }
    }
    if let Some(object) = value.as_object() {
        if schema["minProperties"]
            .as_u64()
            .is_some_and(|min| object.len() < min as usize)
        {
            return false;
        }
        if schema["required"].as_array().is_some_and(|fields| {
            fields
                .iter()
                .any(|key| !object.contains_key(key.as_str().unwrap()))
        }) {
            return false;
        }
        for (key, item) in object {
            let field = &schema["properties"][key];
            if field.is_null() {
                if schema["additionalProperties"] == false {
                    return false;
                }
            } else if !schema_accepts(field, item) {
                return false;
            }
        }
    }
    true
}

#[test]
fn integer_payload_contract_explicitly_rejects_integral_float_representations() {
    for topic in ["apps create", "notify send"] {
        let schema = describe(topic)["schemas"]["payload"].clone();
        let description = schema["description"].as_str().unwrap();
        for restriction in [
            "JSON integer lexical representation",
            "signed i64",
            "decimals and exponents parsed as floats",
            "stricter than standard JSON Schema integer semantics",
        ] {
            assert!(description.contains(restriction), "{topic}: {restriction}");
        }
    }
    for payload in [r#"{"repeat":1.0}"#, r#"{"repeat":1e0}"#] {
        let output = run(&["--json", "apps", "create", "demo", "--payload", payload]);
        assert_eq!(output.status.code(), Some(2));
        let error: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(error["error"]["code"], "ARGUMENT");
    }
}

#[test]
fn all_discoverable_pushed_field_constraints_match_local_validation_even_in_raw_mode() {
    let schema = describe("apps create")["schemas"]["payload"].clone();
    let properties = schema["properties"].as_object().unwrap();
    assert_eq!(properties.len(), 35);
    assert_eq!(schema, describe("apps update")["schemas"]["payload"]);
    let probes = vec![
        json!(null),
        json!(false),
        json!(0),
        json!(-1),
        json!(129),
        json!(0.1),
        json!(1.0),
        json!(11.0),
        json!("x"),
        json!([]),
        json!([1, 2]),
        json!(["x"]),
        json!({}),
        json!({"mode":"bounce","direction":"right","entry":"offscreen","whenFits":"scroll","speed":0}),
        json!({"speed":-1}),
        json!({"extra":true}),
        json!([{"icon":"mail","x":65535}]),
        json!([{"icon":"mail","x":65536}]),
        json!([{"x":0}]),
        json!([{"icon":"mail","extra":true}]),
        json!(vec![1; 17]),
    ];
    for (field, constraint) in properties {
        let mut candidates = probes.clone();
        if let Some(values) = constraint["enum"].as_array() {
            candidates.extend(values.iter().cloned());
        }
        for candidate in candidates {
            let payload = json!({(field):candidate}).to_string();
            let output = run(&[
                "--json",
                "--target",
                "http://127.0.0.1:1",
                "apps",
                "create",
                "demo",
                "--payload",
                &payload,
                "--raw",
            ]);
            let actual: Value = serde_json::from_slice(&output.stdout).unwrap();
            let accepted_locally = actual["error"]["code"] != "ARGUMENT";
            assert_eq!(
                accepted_locally,
                schema_accepts(constraint, &candidate),
                "{field}: {candidate}; {actual}"
            );
            if accepted_locally {
                assert_eq!(actual["error"]["code"], "TRANSPORT");
            }
        }
    }
}

#[test]
fn payload_sources_are_exclusive_and_raw_only_bypasses_unknown_keys() {
    for prefix in [
        vec!["apps", "create", "demo"],
        vec!["apps", "update", "demo"],
        vec!["notify", "send"],
    ] {
        for suffix in [
            vec![],
            vec!["--payload", r#"{"text":"x"}"#, "--file", "unused.json"],
            vec!["--payload", r#"{"vendor":1}"#],
            vec!["--raw", "--payload", r#"{"iconGap":129,"vendor":1}"#],
        ] {
            let mut args = vec!["--json", "--target", "http://127.0.0.1:1"];
            args.extend_from_slice(&prefix);
            args.extend_from_slice(&suffix);
            let output = run(&args);
            assert_eq!(output.status.code(), Some(2), "{args:?}");
            let error: Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(error["error"]["code"], "ARGUMENT");
        }
        let mut args = prefix.clone();
        args.extend(["--raw", "--payload", r#"{"vendor":1}"#]);
        let notify = prefix[0] == "notify";
        let (_, body) = exchange(
            &args,
            if notify { "POST" } else { "PUT" },
            if notify {
                "/api/v1/notifications"
            } else {
                "/api/v1/apps/pushed/demo"
            },
            json!({"ok":true}),
        );
        assert_eq!(body["vendor"], 1);
    }
}

#[test]
fn deletion_discovery_never_offers_payload_input_and_its_example_executes() {
    let description = describe("apps delete");
    assert!(description["parameters"]["--payload"].is_null());
    assert!(description["schemas"]["payload"].is_null());
    assert!(description["schemas"]["input"].is_null());
    for example in description["examples"].as_array().unwrap() {
        let args: Vec<&str> = example
            .as_str()
            .unwrap()
            .split_whitespace()
            .skip(1)
            .filter(|arg| *arg != "--json")
            .collect();
        let (actual, body) = exchange(&args, "DELETE", "/api/v1/apps/build", json!({"ok":true}));
        assert_eq!(actual["operation"], "delete");
        assert_eq!(body, Value::Null);
    }
}

// These command examples use whitespace and single-quoted JSON, with no shell expansion.
fn example_args(example: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut word = String::new();
    let mut quoted = false;
    for ch in example.chars() {
        match ch {
            '\'' => quoted = !quoted,
            ch if ch.is_whitespace() && !quoted => {
                if !word.is_empty() {
                    args.push(std::mem::take(&mut word));
                }
            }
            ch => word.push(ch),
        }
    }
    assert!(!quoted, "unclosed quote: {example}");
    if !word.is_empty() {
        args.push(word);
    }
    args
}

#[test]
fn published_payload_examples_execute_with_one_source_and_valid_schema_content() {
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), r#"{"text":"from file"}"#).unwrap();
    for topic in ["apps create", "apps update", "notify send"] {
        let description = describe(topic);
        for example in description["examples"].as_array().unwrap() {
            let words = example_args(example.as_str().unwrap());
            assert_eq!(words[0], "awtrix-cli");
            let mut args = Vec::new();
            let mut words = words.into_iter().skip(1);
            while let Some(word) = words.next() {
                match word.as_str() {
                    "--json" => {}
                    "--target" => {
                        words.next().unwrap();
                    }
                    "payload.json" => args.push(file.path().to_str().unwrap().to_owned()),
                    _ => args.push(word),
                }
            }
            assert_eq!(
                args.iter()
                    .filter(|arg| matches!(arg.as_str(), "--payload" | "--file"))
                    .count(),
                1
            );
            let argv: Vec<&str> = args.iter().map(String::as_str).collect();
            let notify = topic == "notify send";
            let (_, body) = exchange(
                &argv,
                if notify { "POST" } else { "PUT" },
                if notify {
                    "/api/v1/notifications"
                } else {
                    "/api/v1/apps/pushed/build"
                },
                json!({"ok":true}),
            );
            // Notification name's pattern and reserved value are separately covered by validation tests.
            assert!(
                schema_accepts(&description["schemas"]["payload"], &body),
                "{topic}: {body}"
            );
        }
    }
}
