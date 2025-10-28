use std::process::Command;
fn main() {
    askama_derive::rerun_if_changed("templates");
    let hash = Command::new("git")
    .args(&["rev-parse", "HEAD"])
    .output()
    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    .unwrap_or_default();
    let branch = Command::new("git")
        .args(&["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    let tag = Command::new("git")
        .args(&["describe", "--tags", "--abbrev=0"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    let rustc_version = Command::new("rustc")
        .arg("--version")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let build_time = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(dur) => format!("{}", dur.as_secs()),
        Err(_) => "unknown".to_string(),
    };
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_else(|_| "unknown".to_string());
    let target_arch =
        std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_else(|_| "unknown".to_string());
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "unknown".to_string());
    let pkg_version = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "unknown".to_string());
    let pkg_name = std::env::var("CARGO_PKG_NAME").unwrap_or_else(|_| "unknown".to_string());

    println!("cargo:rustc-env=RUSTC_VERSION={}", rustc_version);
    println!("cargo:rustc-env=BUILD_TIME={}", build_time);
    println!("cargo:rustc-env=CARGO_CFG_TARGET_OS={}", target_os);
    println!("cargo:rustc-env=CARGO_CFG_TARGET_ARCH={}", target_arch);
    println!("cargo:rustc-env=PROFILE={}", profile);
    println!("cargo:rustc-env=CARGO_PKG_VERSION={}", pkg_version);
    println!("cargo:rustc-env=CARGO_PKG_NAME={}", pkg_name);
    println!("cargo:rustc-env=GIT_HASH={}", hash);
    println!("cargo:rustc-env=GIT_BRANCH={}", branch);
    println!("cargo:rustc-env=GIT_TAG={}", tag);
}

