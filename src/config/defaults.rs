// ===== 应用运行时常量 =====
pub const APP_ID: &str = "com.jialuo.niripet";
pub const CAROUSEL_INTERVAL_MS: u64 = 130;
pub const INPUT_DEBUG_LOG: bool = false;
pub const DRAG_LONG_PRESS_MS: u64 = 450;
pub const DRAG_ALLOW_OFFSCREEN: bool = true;

// ===== 动画资源路径默认配置 =====
pub const ASSETS_BODY_ROOT: &str = "assets/body";
pub const DEFAULT_HAPPY_IDLE_VARIANTS: &[&str] = &[
	"Default/Happy",
];
pub const DEFAULT_NOMAL_IDLE_ROOT: &str = "Default/Nomal";
pub const DEFAULT_POOR_CONDITION_IDLE_ROOT: &str = "Default/PoorCondition";
pub const DEFAULT_ILL_IDLE_ROOT: &str = "Default/Ill";
pub const STARTUP_ROOT: &str = "StartUP";
pub const RAISE_DYNAMIC_ROOT: &str = "Raise/Raised_Dynamic";
pub const RAISE_STATIC_ROOT: &str = "Raise/Raised_Static";
pub const PINCH_ROOT: &str = "Pinch";
pub const SHUTDOWN_ROOT: &str = "Shutdown";
pub const TOUCH_HEAD_ROOT: &str = "Touch_Head";
pub const TOUCH_BODY_ROOT: &str = "Touch_Body";

// ===== 面板调试默认值 =====
pub const PANEL_BASIC_STAT_MAX: u32 = 100;
pub const PANEL_EXPERIENCE_MAX: u32 = 100;

pub const PANEL_DEFAULT_STAMINA: u32 = 80;
pub const PANEL_DEFAULT_SATIETY: u32 = 70;
pub const PANEL_DEFAULT_THIRST: u32 = 65;
pub const PANEL_DEFAULT_MOOD: u32 = 75;
pub const PANEL_DEFAULT_HEALTH: u32 = 90;
pub const PANEL_DEFAULT_AFFINITY: u32 = 50;
pub const PANEL_DEFAULT_EXPERIENCE: u32 = 10;
pub const PANEL_DEFAULT_LEVEL: u32 = 3;

// ===== 运行时配置文件 =====
pub const RUNTIME_CONFIG_FILE: &str = "config.toml";
