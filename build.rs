use std::env;
use std::fs;
use std::path::PathBuf;

const SYS_CRATE: &str = "basis-universal-sys-0.3.1";

fn find_transcoder_headers() -> PathBuf {
    let cargo_home = env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".cargo")))
        .expect("CARGO_HOME or HOME must be set to locate basis-universal-sys");
    let sources = cargo_home.join("registry").join("src");
    fs::read_dir(&sources)
        .expect("cargo registry source directory is missing; run cargo fetch")
        .filter_map(Result::ok)
        .map(|entry| entry.path().join(SYS_CRATE).join("vendor/basis_universal/transcoder"))
        .find(|path| path.join("basisu_transcoder.h").is_file())
        .unwrap_or_else(|| panic!("{SYS_CRATE} transcoder headers not found under {}", sources.display()))
}

fn main() {
    let headers = find_transcoder_headers();
    println!("cargo:rerun-if-changed=native/basislz_ktx2.cpp");
    cc::Build::new()
        .cpp(true)
        .std("c++11")
        .file("native/basislz_ktx2.cpp")
        .include(&headers)
        .define("BASISD_SUPPORT_KTX2_ZSTD", "0")
        .flag_if_supported("-fno-strict-aliasing")
        .flag_if_supported("-Wno-unused-value")
        .flag_if_supported("-Wno-unused-parameter")
        .compile("bevy_basislz_ktx2");
}
