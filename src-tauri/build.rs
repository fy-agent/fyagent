const WINDOWS_TEST_MANIFEST: &str = include_str!("windows/fyagent-test.manifest");
const WINDOWS_RELEASE_MANIFEST: &str = include_str!("windows/fyagent-release.manifest");

#[derive(Clone, Copy)]
enum WindowsManifest {
    Test,
    Release,
}

impl WindowsManifest {
    fn contents(self) -> &'static str {
        match self {
            Self::Test => WINDOWS_TEST_MANIFEST,
            Self::Release => WINDOWS_RELEASE_MANIFEST,
        }
    }
}

fn main() {
    println!("cargo:rustc-check-cfg=cfg(fyagent_windows_release)");
    println!(
        "cargo:rustc-check-cfg=cfg(fyagent_macos_system_commit_mode, values(\"development\", \"formal\"))"
    );
    emit_privileged_client_link();

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    match target_os.as_str() {
        "macos" => {
            tauri_build::build();
            return;
        }
        "windows" => {}
        _ => {
            tauri_build::build();
            return;
        }
    }

    println!("cargo:rerun-if-env-changed=FYAGENT_WINDOWS_MANIFEST");
    println!("cargo:rerun-if-changed=windows/fyagent-test.manifest");
    println!("cargo:rerun-if-changed=windows/fyagent-release.manifest");

    let manifest = select_windows_manifest();
    if matches!(manifest, WindowsManifest::Release) {
        // The runtime uses this cfg for formal builds only; test/dev builds
        // must remain able to run without UAC.
        println!("cargo:rustc-cfg=fyagent_windows_release");
    }
    let windows = tauri_build::WindowsAttributes::new().app_manifest(manifest.contents());
    let attributes = tauri_build::Attributes::new().windows_attributes(windows);

    tauri_build::try_build(attributes).expect("failed to embed the FyAgent Windows manifest");
    embed_test_manifest();
}

/// Link the Swift client for explicitly featured macOS bundles. Runtime
/// admission is a separate compile-time decision: production Release bundles
/// can carry and sign the helper/client while keeping system commit disabled
/// until a dedicated HIL candidate opts into the formal mode.
fn emit_privileged_client_link() {
    println!("cargo:rerun-if-env-changed=FYAGENT_PRIVILEGED_CLIENT_DYLIB");
    println!("cargo:rerun-if-env-changed=FYAGENT_MACOS_SYSTEM_COMMIT_MODE");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos")
        || std::env::var_os("CARGO_FEATURE_MACOS_PRIVILEGED_CLIENT").is_none()
    {
        return;
    }

    let mode = match std::env::var("FYAGENT_MACOS_SYSTEM_COMMIT_MODE") {
        Ok(value) if value == "development" || value == "formal" => Some(value),
        Ok(value) => panic!(
            "invalid FYAGENT_MACOS_SYSTEM_COMMIT_MODE={value:?}; expected development or formal"
        ),
        Err(std::env::VarError::NotPresent) => None,
        Err(error) => panic!("failed to read FYAGENT_MACOS_SYSTEM_COMMIT_MODE: {error}"),
    };

    let manifest_dir = std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("missing CARGO_MANIFEST_DIR"),
    );
    let mut candidates = Vec::new();
    if let Ok(explicit) = std::env::var("FYAGENT_PRIVILEGED_CLIENT_DYLIB") {
        if !explicit.is_empty() {
            candidates.push(std::path::PathBuf::from(explicit));
        }
    }
    candidates
        .push(manifest_dir.join("macos-privileged-helper/dist/libFyAgentPrivilegedClient.dylib"));
    candidates.push(
        manifest_dir
            .join("macos-privileged-helper/dist/development/libFyAgentPrivilegedClient.dylib"),
    );
    candidates.push(manifest_dir.join(
        "macos-privileged-helper/.build/apple/Products/Release/libFyAgentPrivilegedClient.dylib",
    ));
    candidates.push(
        manifest_dir
            .join("macos-privileged-helper/.build/release/libFyAgentPrivilegedClient.dylib"),
    );

    let mut selected = None;
    for candidate in candidates {
        println!("cargo:rerun-if-changed={}", candidate.display());
        let Ok(metadata) = std::fs::symlink_metadata(&candidate) else {
            continue;
        };
        if metadata.is_file() && !metadata.file_type().is_symlink() {
            selected = Some(candidate);
            break;
        }
    }

    let selected = selected.unwrap_or_else(|| {
        panic!("macOS privileged client feature is enabled, but libFyAgentPrivilegedClient.dylib is missing")
    });
    let parent = selected
        .parent()
        .expect("privileged client dylib must have a parent directory");
    if let Some(mode) = mode {
        println!("cargo:rustc-cfg=fyagent_macos_system_commit_mode=\"{mode}\"");
    }
    println!("cargo:rustc-link-search=native={}", parent.display());
    println!("cargo:rustc-link-lib=dylib=FyAgentPrivilegedClient");
    println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/../Frameworks");
}

/// Cargo's test-only link arguments do not reach the library unit-test harness.
/// Emit the ordinary-privilege manifest generically so unit and integration
/// harnesses receive Common Controls v6, then disable linker-generated
/// manifests for application binaries. The application keeps the manifest
/// resource selected and embedded by `tauri-build` above.
fn embed_test_manifest() {
    let manifest_path = std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("missing CARGO_MANIFEST_DIR"),
    )
    .join("windows/fyagent-test.manifest");

    println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
    println!(
        "cargo:rustc-link-arg=/MANIFESTINPUT:{}",
        manifest_path.display()
    );
    println!("cargo:rustc-link-arg-bins=/MANIFEST:NO");
}

fn select_windows_manifest() -> WindowsManifest {
    match std::env::var("FYAGENT_WINDOWS_MANIFEST") {
        Ok(value) if value == "release" => WindowsManifest::Release,
        Ok(value) if value == "test" || value == "dev" => WindowsManifest::Test,
        Ok(value) => {
            panic!("invalid FYAGENT_WINDOWS_MANIFEST={value:?}; expected release, test, or dev")
        }
        Err(std::env::VarError::NotPresent)
            if std::env::var("PROFILE").as_deref() == Ok("release") =>
        {
            // `cargo test --release` uses the same profile as a distributable
            // application, but must retain the ordinary-user test manifest.
            // A release application therefore has to opt in explicitly in the
            // signed release workflow; silently selecting either manifest here
            // would make one of those two binaries misleading.
            panic!(
                "FYAGENT_WINDOWS_MANIFEST must be explicitly set to release for a formal Windows release build or test for a release-profile test harness"
            )
        }
        Err(std::env::VarError::NotPresent) => WindowsManifest::Test,
        Err(error) => panic!("failed to read FYAGENT_WINDOWS_MANIFEST: {error}"),
    }
}
