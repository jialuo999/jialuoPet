// ===== stats 子模块声明 =====
pub mod food;
pub mod leveling;
mod save;
mod model;
mod service;

// ===== 对外导出 =====
pub use model::{InteractType, PetMode, PetRuntimeState, PetStats, StudyMode};
pub use save::PetStatsSaveStore;
pub use service::PetStatsService;
