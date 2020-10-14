use super::{level_var, tracked_flag, tracked_var};
// fn tracked_var(s: &str) -> Option<String> {
//     println!("cargo:rerun-if-env-changed={}", s);
//     std::env::var(s).ok()
// }
// fn tracked_flag(s: &str) -> Option<bool> {
//     match tracked_var(s)?.to_ascii_lowercase() {
//         // matches handling in options.c
//         "1" | "true" | "yes" | "on" => Some(true),
//         "0" | "false" | "no" | "off" => Some(false),
//         "" => None,
//         n => {
//             println!("cargo:warning=unknown {} value: {:?}", s, n);
//         },
//     }
// }

fn override_supported(target: &str) -> bool {
    // Apple prevents this more than anybody else.
    !(target.contains("apple") || target.contains("darwin")) &&
    // Windows allows it only for shared libraries (even then only in some cases)
    !target.contains("windows") &&
    // "mimalloc is not working correctly on DragonFly yet" (this is just for overriding)
    // https://github.com/microsoft/mimalloc/blob/13a4030619/include/mimalloc-internal.h#L307
    !target.contains("dragonfly") &&
    // "goes wrong if anyone uses names > 23 characters"
    // https://github.com/microsoft/mimalloc/blob/13a4030619/include/mimalloc-internal.h#L303
    !target.contains("openbsd")
    // TARGETS NEEDING TESTING (caused issues with jemallocs overrides in the past):
    // - android
    // - musl
}

pub fn build() {
    // only use for vars guaranteed by cargo.
    fn getenv(s: &str) -> String {
        std::env::var(s).unwrap()
    }
    fn testenv(s: &str) -> bool {
        std::env::var(s).is_ok()
    }
    let target = getenv("TARGET");
    let target_family = getenv("CARGO_CFG_TARGET_FAMILY");
    let profile = getenv("PROFILE");
    let windows = target_family == "windows";

    let mut build = cc::Build::new();
    // if testenv("CARGO_FEATURE_COMPILE_AS_CXX") {
    //     build.cpp(true).cpp_link_stdlib(None);
    // }
    let compiler = build.get_compiler();

    let path = tracked_var("RUST_MI_SOURCEDIR").unwrap_or("c_src/mimalloc".into());
    let root = std::path::PathBuf::from(path);

    build
        .include(root.join("src"))
        .include(root.join("include"))
        .file(root.join("src/static.c"));

    let force_secure = level_var("RUST_MI_SECURE", 4);
    let secure = force_secure.or_else(|| std::env::var("CARGO_FEATURE_SECURE").ok().map(|_| 4));

    if let Some(n) = secure {
        build.define("MI_SECURE", Some(n.to_string().as_ref()));
    }

    // let debug_level = level_var("RUST_MI_DEBUG", 3).unwrap_or_else(|| (profile == "debug") as u8);
    // let debug_level = if testenv("CARGO_FEATURE_DEBUG_FULL") {
    //     3
    // } else {
    //     (profile == "debug") as u8
    // };
    let debug_level = if testenv("CARGO_FEATURE_DEBUG_FULL") {
        3
    } else {
        let ol = getenv("OPT_LEVEL");
        let dbg = if let "2" | "3" | "s" | "z" = ol.as_str() {
            false
        } else {
            profile == "debug" || profile == "dev"
        };
        dbg as u8
    };
    if debug_level == 0 {
        build.define("NDEBUG", None);
    } else {
        build.define("MI_DEBUG", debug_level.to_string().as_ref());
    }

    if let Some(s) = level_var("RUST_MI_STAT", 2) {
        build.define("MI_STAT", s.to_string().as_str());
    }

    if testenv("CARGO_FEATURE_ABORT_ON_ALLOC_FAIL") {
        build.define("MI_XMALLOC", "1");
        println!("cargo:rustc-cfg=mi_xmalloc");
    }

    let padding = level_var("RUST_MI_PADDING", 2).unwrap_or_default();
    build.define("MI_PADDING", padding.to_string().as_str());

    let really_override = tracked_flag("RUST_MI_FORCE_OVERRIDE") == Some(true);
    let override_requested = testenv("CARGO_FEATURE_UNSAFE_OVERRIDE_LIBC_ALLOCATORS");
    let do_override = really_override || (override_requested && override_supported(&target));
    if do_override {
        build.define("MI_OVERRIDE", None);
        let apple = target.contains("apple") || target.contains("darwin");
        if really_override && apple {
            // better or worse the cmake defaults this on
            let interpose = tracked_flag("RUST_MI_INTERPOSE").unwrap_or(true);
            let osxzone = tracked_flag("RUST_MI_OSX_ZONE").unwrap_or_default();
            if osxzone || interpose {
                build.define("MI_INTERPOSE", None);
            }
            if osxzone {
                build.define("MI_OSX_ZONE", "1");
            }
        }
    }

    if !compiler.is_like_msvc() && !windows {
        if !target.contains("haiku") {
            if testenv("CARGO_FEATURE_LOCAL_DYNAMIC_TLS") {
                build.flag("-ftls-model=local-dynamic");
            } else {
                build.flag("-ftls-model=initial-exec");
            }
        }
        build.flag("-fvisibility=hidden");
    }

    if let Some(flags) = tracked_var("RUST_MI_EXTRA_CFLAGS") {
        for flag in flags.split_ascii_whitespace() {
            build.flag(flag);
        }
    }
    build.compile("libmimalloc.a");
}
