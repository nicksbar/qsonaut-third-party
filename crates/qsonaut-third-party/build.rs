use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=RADE_C_DIR");
    println!("cargo:rerun-if-env-changed=RADE_C_BUILD_DIR");
    println!("cargo:rerun-if-env-changed=RADE_C_LIB_DIR");
    println!("cargo:rerun-if-changed=src/rade/speech_bridge.c");

    if env::var_os("CARGO_FEATURE_RADE_C").is_none() {
        return;
    }

    let build_dir = env::var_os("RADE_C_BUILD_DIR")
        .map(PathBuf::from)
        .or_else(|| env::var_os("RADE_C_DIR").map(|dir| PathBuf::from(dir).join("build")));
    let lib_dir = env::var_os("RADE_C_LIB_DIR")
        .map(PathBuf::from)
        .or_else(|| build_dir.as_ref().map(|dir| dir.join("src")));

    match lib_dir {
        Some(path) => {
            println!("cargo:rustc-link-search=native={}", path.display());
            println!("cargo:rustc-link-lib=dylib=rade");

            if env::var_os("CARGO_FEATURE_RADE_SPEECH").is_some() {
                let Some(rade_dir) = env::var_os("RADE_C_DIR").map(PathBuf::from) else {
                    panic!("rade-speech requires RADE_C_DIR so the upstream FARGAN/LPCNet bridge can be built");
                };
                let build_dir = build_dir.unwrap_or_else(|| rade_dir.join("build"));
                let opus_root = build_dir.join("build_opus-prefix/src/build_opus");
                let opus_lib = opus_root.join(".libs");
                let opus_dnn = opus_root.join("dnn");
                let opus_include = opus_root.join("include");
                let opus_celt = opus_root.join("celt");
                cc::Build::new()
                    .file("src/rade/speech_bridge.c")
                    .include(&opus_dnn)
                    .include(&opus_include)
                    .include(&opus_celt)
                    .warnings(true)
                    .compile("qsonaut_rade_speech_bridge");
                println!("cargo:rustc-link-search=native={}", opus_lib.display());
                println!("cargo:rustc-link-lib=static=opus");
                println!("cargo:rustc-link-lib=m");
            }
        }
        None => {
            println!(
                "cargo:warning=rade-c enabled without RADE_C_LIB_DIR or RADE_C_DIR; native linking will require an external rade library"
            );
        }
    }
}
