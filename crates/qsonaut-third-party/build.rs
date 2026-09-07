use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=RADE_C_DIR");
    println!("cargo:rerun-if-env-changed=RADE_C_BUILD_DIR");
    println!("cargo:rerun-if-env-changed=RADE_C_LIB_DIR");
    println!("cargo:rerun-if-changed=src/rade/speech_bridge.c");

    let mut rade_dir = env::var_os("RADE_C_DIR").map(PathBuf::from);
    let mut build_dir = env::var_os("RADE_C_BUILD_DIR").map(PathBuf::from);
    let mut lib_dir = env::var_os("RADE_C_LIB_DIR").map(PathBuf::from);

    if rade_dir.is_none() {
        let helper = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("../../tools/build-rade-c.sh");
        // GitHub's Windows runners provide Bash through Git for Windows, but
        // Windows cannot execute a `.sh` file directly. Invoke the helper
        // through Bash on Windows while preserving direct execution on Unix.
        let mut command = if cfg!(windows) {
            let mut command = Command::new("bash");
            command.arg(&helper);
            command
        } else {
            Command::new(&helper)
        };
        let output = command
            .output()
            .unwrap_or_else(|error| panic!("failed to run {}: {error}", helper.display()));
        if !output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let details = [stdout, stderr]
                .into_iter()
                .filter(|text| !text.is_empty())
                .collect::<Vec<_>>()
                .join("\n");
            panic!(
                "bundled RADE build failed{}",
                if details.is_empty() {
                    ".".to_string()
                } else {
                    format!(":\n{details}")
                }
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
            if cfg!(unix) {
                println!("cargo:rustc-link-arg=-Wl,-rpath,{}", path.display());
            }

            let rade_dir = rade_dir.expect("bundled RADE build did not report RADE_C_DIR");
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
            if cfg!(unix) {
                println!("cargo:rustc-link-lib=m");
            }
        }
        None => {
            println!("bundled RADE build did not provide RADE_C_LIB_DIR or RADE_C_DIR");
        }
    }
}
