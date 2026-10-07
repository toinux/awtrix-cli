//! Opt-in system integration against the actual AWTRIX Linux executable.
#[cfg(target_os = "linux")]
use serde_json::Value;
#[cfg(target_os = "linux")]
use std::{
    net::TcpStream,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::Duration,
};

#[cfg(target_os = "linux")]
fn run(manifest: &str, binary: &Path, webui: &Path, config: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_awtrix"))
        .args([
            "--json",
            "test",
            "project",
            "--manifest",
            manifest,
            "--binary",
            binary.to_str().unwrap(),
            "--webui",
            webui.to_str().unwrap(),
        ])
        .env("AWTRIX_CONFIG", config)
        .env_remove("AWTRIX_URL")
        .env_remove("AWTRIX_PROFILE")
        .output()
        .unwrap()
}

#[cfg(target_os = "linux")]
fn parse(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|_| {
        panic!(
            "expected structured test result; stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

#[cfg(target_os = "linux")]
fn target_is_stopped(target: &str) -> bool {
    let address = target.strip_prefix("http://").unwrap();
    TcpStream::connect_timeout(&address.parse().unwrap(), Duration::from_millis(250)).is_err()
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "requires the supplied AWTRIX Linux executable and web UI; run with --ignored"]
fn real_awtrix_headless_declarative_runs_cover_success_failure_berry_error_and_isolation() {
    let binary = std::env::var_os("AWTRIX_LINUX_BIN")
        .map(PathBuf::from)
        .expect("set AWTRIX_LINUX_BIN to the supplied real awtrix-linux executable");
    let webui = std::env::var_os("AWTRIX_WEBUI")
        .map(PathBuf::from)
        .expect("set AWTRIX_WEBUI to the AWTRIX webui/index.html file");
    assert!(binary.is_file(), "AWTRIX_LINUX_BIN must be a file");
    assert!(webui.is_file(), "AWTRIX_WEBUI must be a file");

    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("config.json");
    let ownership = config.with_file_name("headless.json");
    let user_record = b"keep the user's headless ownership unchanged";
    std::fs::write(&ownership, user_record).unwrap();

    let mut successful_targets = Vec::new();
    let mut data_paths = Vec::new();
    for _ in 0..2 {
        let output = run("examples/project/awtrix.toml", &binary, &webui, &config);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let result = parse(&output);
        assert_eq!(result["target_mode"], "isolated-headless");
        assert_eq!(
            result["deployment"]["succeeded"],
            serde_json::json!([
                "module:helpers",
                "resource:/ICONS/project.gif",
                "script:main"
            ])
        );
        assert!(result["assertions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item["passed"] == true));
        assert_eq!(result["observed_window"]["complete"], true);
        successful_targets.push(result["target"].as_str().unwrap().to_owned());
        data_paths.push(PathBuf::from(
            result["isolation"]["data_path"].as_str().unwrap(),
        ));
    }
    assert_ne!(successful_targets[0], successful_targets[1]);

    let failed = run(
        "examples/project-failing-assertion/awtrix.toml",
        &binary,
        &webui,
        &config,
    );
    assert_eq!(failed.status.code(), Some(1));
    let failed_result = parse(&failed);
    assert_eq!(failed_result["assertions"][0]["passed"], false);
    data_paths.push(PathBuf::from(
        failed_result["isolation"]["data_path"].as_str().unwrap(),
    ));

    let berry = run(
        "examples/project-berry-error/awtrix.toml",
        &binary,
        &webui,
        &config,
    );
    assert_eq!(berry.status.code(), Some(1));
    let berry_result = parse(&berry);
    assert!(berry_result["errors"]
        .as_array()
        .unwrap()
        .iter()
        .any(|error| {
            error["code"] == "BERRY_ERROR"
                && error["runtime_error"]["hook"] == "loop"
                && error["runtime_error"]["message"]
                    .as_str()
                    .is_some_and(|message| message.contains("declarative test runtime failure"))
        }));
    data_paths.push(PathBuf::from(
        berry_result["isolation"]["data_path"].as_str().unwrap(),
    ));

    let additional_success = parse(&run(
        "examples/project/awtrix.toml",
        &binary,
        &webui,
        &config,
    ));
    data_paths.push(PathBuf::from(
        additional_success["isolation"]["data_path"]
            .as_str()
            .unwrap(),
    ));
    for result in [additional_success, failed_result, berry_result] {
        assert!(target_is_stopped(result["target"].as_str().unwrap()));
    }
    for target in successful_targets {
        assert!(target_is_stopped(&target));
    }
    assert_eq!(
        data_paths
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        data_paths.len()
    );
    assert!(
        data_paths.iter().all(|path| !path.exists()),
        "each run's isolated data directory must be removed"
    );
    assert_eq!(std::fs::read(&ownership).unwrap(), user_record);

    // Exercise the exact public visual-test path against real Berry rendering, not a mocked
    // framebuffer. The script clears each frame and draws one fixed pixel on this headless panel.
    let visual = root.path().join("visual-project");
    std::fs::create_dir_all(visual.join("src")).unwrap();
    std::fs::create_dir_all(visual.join("reference")).unwrap();
    std::fs::write(visual.join("src/main.be"), "# @name main\nclass Main\n  def draw()\n    clear()\n    pixel(0, 0, 0xFF0000)\n  end\nend\nreturn Main()\n").unwrap();
    let reference = visual.join("reference/red-pixel.png");
    let mut rgb = vec![0_u8; 52 * 16 * 3];
    rgb[..3].copy_from_slice(&[255, 0, 0]);
    let file = std::fs::File::create(&reference).unwrap();
    let mut encoder = png::Encoder::new(file, 52, 16);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&rgb)
        .unwrap();
    std::fs::write(visual.join("awtrix.toml"), "[project]\nname='visual-headless'\nversion='1'\n[[scripts]]\nname='main'\nfile='src/main.be'\ncreate=true\n[tests]\nwindow_secs=2\n[[tests.assertion]]\nname='stable-red-pixel'\nreference='reference/red-pixel.png'\nmax_channel_diff=0\nmax_different_pixels=0\nselect_app='main'\n").unwrap();
    let visual_manifest = visual.join("awtrix.toml");
    let visual_output = run(visual_manifest.to_str().unwrap(), &binary, &webui, &config);
    assert_eq!(
        visual_output.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&visual_output.stdout),
        String::from_utf8_lossy(&visual_output.stderr)
    );
    let visual_report = parse(&visual_output);
    assert_eq!(visual_report["assertions"][0]["passed"], true);
    assert_eq!(
        visual_report["assertions"][0]["observed"]["different_pixels"],
        0
    );
    assert!(PathBuf::from(
        visual_report["assertions"][0]["observed"]["capture"]["path"]
            .as_str()
            .unwrap()
    )
    .is_file());
    assert!(PathBuf::from(
        visual_report["assertions"][0]["observed"]["diff"]["path"]
            .as_str()
            .unwrap()
    )
    .is_file());
    assert!(visual_report["assertions"][0]["observed"]
        .get("pixels")
        .is_none());
}
