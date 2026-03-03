// ===== stats 子模块声明 =====
pub mod food;
pub mod leveling;
mod model;
mod service;

// ===== 对外导出 =====
pub use model::{InteractType, PetMode};
pub use service::PetStatsService;
