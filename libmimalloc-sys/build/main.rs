pub mod cmake;
pub mod cc;

pub fn user_var(s: &str) -> Option<String> {
    println!("cargo:rerun-if-env-changed={}", s);
    std::env::var(s).ok()
}

pub fn user_flag(s: &str) -> Option<bool> {
    match user_var(s)?.as_str() {
        "0" | "false" => Some(false),
        "" => None,
        _ => Some(true),
    }
}
fn can_use_cmake() -> bool {
    // Check if we can locate cmake.
    let cmake = std::env::var("CMAKE").unwrap_or_else(|_| "cmake".to_string());
    match std::process::Command::new(&cmake).arg("--version").output() {
        Ok(v) if v.status.success() => {},
        _ => return false,
    }
    // We know where cmake is, but we still can't really use it for
    // cross-compile or static-crt cases.
    if std::env::var("HOST").unwrap() != std::env::var("TARGET").unwrap() {
        return false;
    }
    if let Ok(v) = std::env::var("CARGO_CFG_TARGET_FEATURE") {
        if v.contains("+static-crt") {
            return false;
        }
    }
    // We *can* use it, it may not be advisable though
    true
}

fn main() {
    if cfg!(feature = "prefer_cmake") && can_use_cmake() {
        cmake::build();
    } else {
        cc::build();
    }
}
