pub const APP_ID: &str = "com.jialuo.niripet";
pub const CAROUSEL_INTERVAL_MS: u64 = 130;
pub const INPUT_DEBUG_LOG: bool = false;
pub const DRAG_LONG_PRESS_MS: u64 = 450;
pub const DRAG_ALLOW_OFFSCREEN: bool = true;
pub const STARTUP_EXCLUDED_DIRS: &[&str] = &["PoorCondition", "Ill"];

pub const ASSETS_BODY_ROOT: &str = "assets/body";
pub const DEFAULT_HAPPY_IDLE_VARIANTS: &[&str] = &[
	"Default/Happy/1",
	"Default/Happy/2",
	"Default/Happy/3",
];
pub const STARTUP_ROOT: &str = "StartUP";
pub const RAISE_DYNAMIC_ROOT: &str = "Raise/Raised_Dynamic";
pub const RAISE_STATIC_ROOT: &str = "Raise/Raised_Static";
pub const SHUTDOWN_VARIANTS: &[&str] = &[
	"Shutdown/2/Happy",
	"Shutdown/Happy_1",
];

// 调试：面板数值控制
pub const PANEL_BASIC_STAT_MAX: u32 = 100;
pub const PANEL_EXPERIENCE_MAX: u32 = 100;
pub const PANEL_LEVEL_MAX: u32 = 100;

pub const PANEL_DEFAULT_STAMINA: u32 = 80;
pub const PANEL_DEFAULT_SATIETY: u32 = 70;
pub const PANEL_DEFAULT_THIRST: u32 = 65;
pub const PANEL_DEFAULT_MOOD: u32 = 75;
pub const PANEL_DEFAULT_HEALTH: u32 = 90;
pub const PANEL_DEFAULT_AFFINITY: u32 = 50;
pub const PANEL_DEFAULT_EXPERIENCE: u32 = 10;
pub const PANEL_DEFAULT_LEVEL: u32 = 3;