use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub struct WindowPosition {
    pub left: i32,
    pub top: i32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct AppSettings {
    pub remember_position: bool,
    pub window_position: Option<WindowPosition>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            remember_position: true,
            window_position: None,
        }
    }
}
