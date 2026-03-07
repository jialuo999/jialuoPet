// ===== 依赖导入 =====
use serde::{Deserialize, Serialize};

// ===== 窗口位置模型 =====
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub struct WindowPosition {
    pub left: i32,
    pub top: i32,
}

// ===== 应用设置模型 =====
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct AppSettings {
    pub remember_position: bool,
    pub window_position: Option<WindowPosition>,
    pub scale_factor: f64,
    pub auto_close_panels_on_outside_click: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            remember_position: true,
            window_position: None,
            scale_factor: 1.0,
            auto_close_panels_on_outside_click: false,
        }
    }
}
