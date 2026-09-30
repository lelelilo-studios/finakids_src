//! Launch configuration (command line / URL query options).

#[derive(Clone, Debug)]
pub struct GameConfig {
    pub save_dir: Option<String>,
    /// Debug: start directly in a location.
    pub scene: Option<String>,
    /// Debug: force time of day (hours).
    pub hour: Option<f32>,
    /// Debug: camera preset for screenshots.
    pub cam: Option<String>,
    /// Debug: jump to a story beat.
    pub beat: Option<String>,
    pub ui_scale: f32,
    pub no_ui: bool,
    pub fresh: bool,
    /// Debug: start a new game immediately.
    pub autostart: bool,
    /// Debug: advance dialogues and pick choices automatically.
    pub autoplay: bool,
    /// Render the app icon instead of the game.
    pub icon: bool,
}

impl Default for GameConfig {
    fn default() -> Self {
        GameConfig {
            save_dir: None,
            scene: None,
            hour: None,
            cam: None,
            beat: None,
            ui_scale: 1.0,
            no_ui: false,
            fresh: false,
            autostart: false,
            autoplay: false,
            icon: false,
        }
    }
}

impl GameConfig {
    /// Applies `--key value` style arguments. Returns true when `value` was consumed.
    pub fn apply_arg(&mut self, key: &str, value: &str) -> bool {
        match key.trim_start_matches('-') {
            "scene" => self.scene = Some(value.to_string()),
            "hour" => self.hour = value.parse().ok(),
            "cam" => self.cam = Some(value.to_string()),
            "beat" => self.beat = Some(value.to_string()),
            "ui-scale" | "ui_scale" => self.ui_scale = value.parse().unwrap_or(1.0),
            "save-dir" => self.save_dir = Some(value.to_string()),
            "no-ui" | "noui" => {
                self.no_ui = true;
                return false;
            }
            "fresh" => {
                self.fresh = true;
                return false;
            }
            "autostart" | "start" => {
                self.autostart = true;
                return false;
            }
            "icon" => {
                self.icon = true;
                return false;
            }
            "autoplay" => {
                self.autoplay = true;
                self.autostart = true;
                return false;
            }
            _ => return false,
        }
        true
    }

    #[cfg(target_arch = "wasm32")]
    pub fn apply_url_query(&mut self) {
        let q = crate::platform::url_query();
        for pair in q.trim_start_matches('?').split('&') {
            let mut it = pair.splitn(2, '=');
            let k = it.next().unwrap_or("");
            let v = it.next().unwrap_or("");
            if k == "noui" || k == "fresh" || k == "start" || k == "autoplay" {
                self.apply_arg(k, "");
            } else if !k.is_empty() {
                self.apply_arg(k, v);
            }
        }
    }
}
