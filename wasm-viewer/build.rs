use std::fs;
use std::path::{Path, PathBuf};

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect(&path, out);
        } else {
            out.push(path);
        }
    }
}

fn main() {
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("../assets/planet");
    println!("cargo:rerun-if-changed={}", root.display());
    let mut files = Vec::new();
    collect(&root, &mut files);
    files.sort();
    let mut table = String::from("pub(crate) static EMBEDDED_PLANET_ASSETS: &[(&str, &[u8])] = &[\n");
    for file in &files {
        let key = file.strip_prefix(&root).unwrap().to_str().unwrap().replace('\\', "/");
        let absolute = fs::canonicalize(file).unwrap();
        table.push_str(&format!("    ({:?}, include_bytes!({:?})),\n", key, absolute.display().to_string()));
    }
    table.push_str("];\n");
    fs::write(PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("embedded_planet_assets.rs"), table).unwrap();
}
