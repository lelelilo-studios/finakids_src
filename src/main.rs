#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut cfg = finakids::GameConfig::default();
    let mut shot: Option<String> = None;
    let (mut w, mut h, mut frames) = (1280u32, 720u32, 90u32);
    let mut i = 1;
    while i < args.len() {
        let next = args.get(i + 1).cloned().unwrap_or_default();
        match args[i].as_str() {
            "--shot" => {
                shot = Some(next);
                i += 1;
            }
            "--size" => {
                if let Some((a, b)) = next.split_once('x') {
                    w = a.parse().unwrap_or(w);
                    h = b.parse().unwrap_or(h);
                }
                i += 1;
            }
            "--frames" => {
                frames = next.parse().unwrap_or(frames);
                i += 1;
            }
            other => match cfg.arg(other, &next) {
                Some(true) => i += 1,
                Some(false) => {}
                None => eprintln!("argumento desconocido: {other}"),
            },
        }
        i += 1;
    }
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    if let Some(path) = shot {
        finakids::platform::init_logger();
        finakids::app::screenshot(cfg, &path, w, h, frames);
        return;
    }
    let _ = (&shot, w, h, frames);
    finakids::run_config(cfg);
}
