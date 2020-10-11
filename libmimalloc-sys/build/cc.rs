

pub fn build() {
    // only use for vars guaranteed by cargo.
    fn getenv(s: &str) -> String {
        std::env::var(s).unwrap()
    }
    let target = getenv("TARGET");
    let target_family = getenv("CARGO_CFG_TARGET_FAMILY");
    // let target_env = getenv("CARGO_CFG_TARGET_ENV");
    let target_os = getenv("CARGO_CFG_TARGET_OS");
    let windows = target_family == "windows";
    // let win_msvc = windows && target_env == "msvc";
    // let win_gnu = windows && target_env == "gnu";

    let mut build = cc::Build::new();
    if cfg!(feature = "compile_cxx") {
        build.cpp(true);
    }
    let compiler = build.get_compiler();

    build.include("c_src/mimalloc/src")
        .include("c_src/mimalloc/include")
        .file("c_src/mimalloc/src/static.c");

    if cfg!(feature = "secure") {
        build.define("MI_SECURE", Some("4"));
    }
    if cfg!(feature = "override") && !windows && !target.contains("dragonfly") {
        build.define("MI_MALLOC_OVERRIDE", None);
        if target_os == "macos" {
            build.define("MI_OSX_ZONE", Some("1"));
            build.define("MI_INTERPOSE", None);
        }
    }
    if cfg!(feature = "show_errors") {
        build.define("MI_SHOW_ERRORS", Some("1"));
    }
    if getenv("PROFILE") == "debug" || cfg!(feature = "full_debug") {
        build.define("MI_DEBUG", Some(if cfg!(feature = "full_debug") { "3" } else { "1" }));
    } else {
        build.define("MI_DEBUG", Some("0"));
        build.define("NDEBUG", None);
    }

    if !compiler.is_like_msvc() && !windows {
        if cfg!(feature = "local_dynamic_tls") {
            build.flag("-ftls-model=local-dynamic");
        } else {
            build.flag("-ftls-model=initial-exec");
        }
        build.flag("-fvisibility=hidden");
    }
    build.compile("libmimalloc.a");
}
