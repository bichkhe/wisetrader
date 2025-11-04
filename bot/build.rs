use std::process::Command;
use std::path::PathBuf;

fn main() {
    // Find git root (go up from bot/ directory to workspace root)
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest_dir.parent().unwrap(); // Go up from bot/ to workspace root
    
    // Get git information from workspace root
    let hash = Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .current_dir(workspace_root)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string());
    
    let branch = Command::new("git")
        .args(&["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(workspace_root)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string());
    
    let tag = Command::new("git")
        .args(&["describe", "--tags", "--abbrev=0"])
        .current_dir(workspace_root)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string());
    
    // Get build time
    let build_time = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(dur) => format!("{}", dur.as_secs()),
        Err(_) => "unknown".to_string(),
    };
    
    // Get target OS
    let target_os = std::env::var("CARGO_CFG_TARGET_OS")
        .unwrap_or_else(|_| "unknown".to_string());
    
    // Set environment variables for compile-time access
    println!("cargo:rustc-env=GIT_HASH={}", hash);
    println!("cargo:rustc-env=GIT_BRANCH={}", branch);
    println!("cargo:rustc-env=GIT_TAG={}", tag);
    println!("cargo:rustc-env=BUILD_TIME={}", build_time);
    println!("cargo:rustc-env=CARGO_CFG_TARGET_OS={}", target_os);
}

