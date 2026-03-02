// ===== settings 子模块声明 =====
mod model;
mod panel;
mod storage;

// ===== 对外导出 =====
pub use model::WindowPosition;
pub use panel::SettingsPanel;
pub use storage::SettingsStore;
