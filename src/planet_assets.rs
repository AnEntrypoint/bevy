use std::io;
use std::path::Path;

pub(crate) fn read(path: &Path) -> io::Result<Vec<u8>> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::fs::read(path)
    }
    #[cfg(target_arch = "wasm32")]
    {
        embedded(path).map(<[u8]>::to_vec).ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, path.display().to_string()))
    }
}

pub(crate) fn exists(path: &Path) -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    {
        path.is_file()
    }
    #[cfg(target_arch = "wasm32")]
    {
        embedded(path).is_some()
    }
}

#[cfg(target_arch = "wasm32")]
include!(concat!(env!("OUT_DIR"), "/embedded_planet_assets.rs"));

#[cfg(target_arch = "wasm32")]
fn embedded(path: &Path) -> Option<&'static [u8]> {
    let text = path.to_str()?.replace('\\', "/");
    let key = text.split_once("assets/planet/")?.1;
    EMBEDDED_PLANET_ASSETS.iter().find(|(name, _)| *name == key).map(|(_, bytes)| *bytes)
}
