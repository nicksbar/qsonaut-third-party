use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=RADE_C_DIR");
    println!("cargo:rerun-if-env-changed=RADE_C_BUILD_DIR");
    println!("cargo:rerun-if-env-changed=RADE_C_LIB_DIR");
    println!("cargo:rerun-if-changed=src/rade/speech_bridge.c");

    if env::var_os("CARGO_FEATURE_RADE_C").is_none() {
        return;
    }

    let bundled = env::var_os("CARGO_FEATURE_RADE_BUNDLED").is_some();
    let mut rade_dir = env::var_os("RADE_C_DIR").map(PathBuf::from);
    let mut build_dir = env::var_os("RADE_C_BUILD_DIR").map(PathBuf::from);
    let mut lib_dir = env::var_os("RADE_C_LIB_DIR").map(PathBuf::from);

    if bundled && rade_dir.is_none() {
        let helper = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("../../tools/build-rade-c.sh");
        let output = Command::new(&helper)
            .output()
            .unwrap_or_else(|error| panic!("failed to run {}: {error}", helper.display()));
        if !output.status.success() {
            panic!(
                "bundled RADE build failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            if let Some(value) = line.strip_prefix("RADE_C_DIR=") {
                rade_dir = Some(PathBuf::from(value));
            } else if let Some(value) = line.strip_prefix("RADE_C_BUILD_DIR=") {
                build_dir = Some(PathBuf::from(value));
            } else if let Some(value) = line.strip_prefix("RADE_C_LIB_DIR=") {
                lib_dir = Some(PathBuf::from(value));
            }
        }
    }

    let build_dir = build_dir.or_else(|| rade_dir.as_ref().map(|dir| dir.join("build")));
    let lib_dir = lib_dir.or_else(|| build_dir.as_ref().map(|dir| dir.join("src")));

    match lib_dir {
        Some(path) => {
            println!("cargo:rustc-link-search=native={}", path.display());
            println!("cargo:rustc-link-lib=dylib=rade");

            if env::var_os("CARGO_FEATURE_RADE_SPEECH").is_some() {
                let Some(rade_dir) = rade_dir else {
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
