use std::env;
use std::process::Command;

fn main() {
    let git_hash = if let Ok(hash) = env::var("GIT_HASH") {
        if hash != "unknown" && !hash.is_empty() {
            hash
        } else {
            get_git_hash_from_command()
        }
    } else {
        get_git_hash_from_command()
    };

    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads/");
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
}

fn get_git_hash_from_command() -> String {
    Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}