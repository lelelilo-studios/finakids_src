//! Platform helpers: logging, persistence and web page integration.

pub fn init_logger() {
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
        let _ = console_log::init_with_level(log::Level::Info);
    }
    #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
    {
        let _ = env_logger::Builder::new()
            .filter_level(log::LevelFilter::Info)
            .filter_module("wgpu_core", log::LevelFilter::Warn)
            .filter_module("wgpu_hal", log::LevelFilter::Warn)
            .filter_module("naga", log::LevelFilter::Warn)
            .parse_default_env()
            .try_init();
    }
}

#[cfg(target_arch = "wasm32")]
pub fn hide_loading() {
    use std::sync::atomic::{AtomicBool, Ordering};
    static DONE: AtomicBool = AtomicBool::new(false);
    if DONE.swap(true, Ordering::Relaxed) {
        return;
    }
    if let Some(el) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("loading"))
    {
        let _ = el.set_attribute("class", "hidden");
    }
}

#[cfg(target_arch = "wasm32")]
pub fn url_query() -> String {
    web_sys::window()
        .and_then(|w| w.location().search().ok())
        .unwrap_or_default()
}

/// Loads a saved string (web: localStorage, native: file in the save dir).
pub fn load_text(dir: Option<&str>, key: &str) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = dir;
        let storage = web_sys::window()?.local_storage().ok()??;
        storage.get_item(key).ok()?
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let path = save_path(dir, key)?;
        std::fs::read_to_string(path).ok()
    }
}

pub fn save_text(dir: Option<&str>, key: &str, value: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = dir;
        if let Some(Ok(Some(storage))) = web_sys::window().map(|w| w.local_storage()) {
            let _ = storage.set_item(key, value);
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Some(path) = save_path(dir, key) {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Err(e) = std::fs::write(&path, value) {
                log::warn!("no se pudo guardar {path:?}: {e}");
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn save_path(dir: Option<&str>, key: &str) -> Option<std::path::PathBuf> {
    use std::path::PathBuf;
    let base: PathBuf = if let Some(d) = dir {
        PathBuf::from(d)
    } else if cfg!(target_os = "ios") {
        PathBuf::from(std::env::var("HOME").ok()?).join("Documents")
    } else if cfg!(target_os = "windows") {
        PathBuf::from(std::env::var("APPDATA").ok()?).join("Finakids")
    } else if cfg!(target_os = "macos") {
        PathBuf::from(std::env::var("HOME").ok()?).join("Library/Application Support/Finakids")
    } else {
        let xdg = std::env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".local/share")))
            .ok()?;
        xdg.join("finakids")
    };
    Some(base.join(format!("{key}.txt")))
}
