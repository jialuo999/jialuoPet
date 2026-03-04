// ===== interaction 子模块声明 =====
mod context_menu;
mod input_probe;
mod input_region;
mod touch_regions;

// ===== 对外导出 =====
pub use context_menu::setup_context_menu;
pub use input_probe::setup_input_probe;
pub use input_region::setup_image_input_region;
pub use touch_regions::{setup_hover_regions, setup_touch_click_regions};
