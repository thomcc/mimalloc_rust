pub mod cc;
pub mod cmake;

pub fn tracked_var(s: &str) -> Option<String> {
    println!("cargo:rerun-if-env-changed={}", s);
    std::env::var(s).ok()
}

pub fn tracked_flag(s: &str) -> Option<bool> {
    match &tracked_var(s)?.to_ascii_lowercase()[..] {
        // matches handling in options.c
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        "" => None,
        n => {
            println!("cargo:warning=unknown {} value: {:?}", s, n);
            None
        }
    }
}
pub fn level_var(s: &str, max: u8) -> Option<u8> {
    match &tracked_var(s)?.to_ascii_lowercase()[..] {
        // matches handling in options.c
        "1" | "true" | "yes" | "on" => Some(1),
        "0" | "false" | "no" | "off" => Some(0),
        "full" => Some(max),
        n if n.parse::<u8>().is_ok() => n.parse::<u8>().ok(),
        "" => None,
        n => {
            println!("cargo:warning=unknown {} value: {:?}", s, n);
            None
        }
    }
}
fn can_use_cmake() -> bool {
    // Check if we can locate cmake.
    let cmake = std::env::var("CMAKE").unwrap_or_else(|_| "cmake".to_string());
    match std::process::Command::new(&cmake).arg("--version").output() {
        Ok(v) if v.status.success() => {}
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
    // let prefer_cmake = std::env::var("CARGO_FEATURE_PREFER_CMAKE").is_ok() && can_use_cmake();
    let explicitly_requested = tracked_var("RUST_MIMALLOC_BUILD_SYSTEM")
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    let want_cmake = std::env::var("CARGO_FEATURE_CMAKE_IF_POSSIBLE").is_ok() && can_use_cmake();
    let use_cmake = match explicitly_requested.as_str() {
        "cmake" => true,
        "cc" => false,
        "" => want_cmake,
        s => {
            println!(
                "cargo:warning=unknown LIB_MIMALLOC_SYS_BUILD_SYSTEM value: {:?}",
                s
            );
            want_cmake
        }
    };

    if use_cmake {
        cmake::build();
    } else {
        cc::build();
    }
}
