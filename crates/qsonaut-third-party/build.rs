use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=RADE_C_DIR");
    println!("cargo:rerun-if-env-changed=RADE_C_LIB_DIR");

    if env::var_os("CARGO_FEATURE_RADE_C").is_none() {
        return;
    }

    let lib_dir = env::var_os("RADE_C_LIB_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("RADE_C_DIR").map(|dir| PathBuf::from(dir).join("build").join("src"))
        });

    match lib_dir {
        Some(path) => {
            println!("cargo:rustc-link-search=native={}", path.display());
            println!("cargo:rustc-link-lib=dylib=rade");
        }
        None => {
            println!(
                "cargo:warning=rade-c enabled without RADE_C_LIB_DIR or RADE_C_DIR; native linking will require an external rade library"
            );
        }
    }
}
