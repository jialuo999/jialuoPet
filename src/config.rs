use anyhow::Context;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub const APP_ID: &str = "com.jialuo.niripet";
pub const CAROUSEL_INTERVAL_MS: u64 = 130;
pub const INPUT_DEBUG_LOG: bool = false;
pub const DRAG_LONG_PRESS_MS: u64 = 450;
pub const DRAG_ALLOW_OFFSCREEN: bool = true;

pub const ASSETS_BODY_ROOT: &str = "assets/body";
pub const DEFAULT_HAPPY_IDLE_VARIANTS: &[&str] = &[
	"Default/Happy/1",
	"Default/Happy/2",
	"Default/Happy/3",
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

pub const RUNTIME_CONFIG_FILE: &str = "config.toml";

static CONFIG_WATCHERS: Lazy<Mutex<Vec<RecommendedWatcher>>> = Lazy::new(|| Mutex::new(Vec::new()));

#[derive(Clone, Debug)]
pub struct PanelDebugConfig {
	pub basic_stat_max: u32,
	pub experience_max: u32,
	pub level_max: u32,
	pub default_stamina: u32,
	pub default_satiety: u32,
	pub default_thirst: u32,
	pub default_mood: u32,
	pub default_health: u32,
	pub default_affinity: u32,
	pub default_experience: u32,
	pub default_level: u32,
}

impl Default for PanelDebugConfig {
	fn default() -> Self {
		Self {
			basic_stat_max: PANEL_BASIC_STAT_MAX,
			experience_max: PANEL_EXPERIENCE_MAX,
			level_max: PANEL_LEVEL_MAX,
			default_stamina: PANEL_DEFAULT_STAMINA,
			default_satiety: PANEL_DEFAULT_SATIETY,
			default_thirst: PANEL_DEFAULT_THIRST,
			default_mood: PANEL_DEFAULT_MOOD,
			default_health: PANEL_DEFAULT_HEALTH,
			default_affinity: PANEL_DEFAULT_AFFINITY,
			default_experience: PANEL_DEFAULT_EXPERIENCE,
			default_level: PANEL_DEFAULT_LEVEL,
		}
	}
}

impl PanelDebugConfig {
	fn sanitized(mut self) -> Self {
		self.basic_stat_max = self.basic_stat_max.max(1);
		self.experience_max = self.experience_max.max(1);
		self.level_max = self.level_max.max(1);

		self.default_stamina = self.default_stamina.min(self.basic_stat_max);
		self.default_satiety = self.default_satiety.min(self.basic_stat_max);
		self.default_thirst = self.default_thirst.min(self.basic_stat_max);
		self.default_mood = self.default_mood.min(self.basic_stat_max);
		self.default_health = self.default_health.min(self.basic_stat_max);
		self.default_affinity = self.default_affinity.min(self.basic_stat_max);
		self.default_experience = self.default_experience.min(self.experience_max);
		self.default_level = self.default_level.min(self.level_max);

		self
	}
}

#[derive(Debug, Default, Deserialize)]
struct FileConfig {
	panel: Option<PanelDebugConfigPartial>,
	animation: Option<AnimationPathConfigPartial>,
}

#[derive(Debug, Default, Deserialize)]
struct PanelDebugConfigPartial {
	basic_stat_max: Option<u32>,
	experience_max: Option<u32>,
	level_max: Option<u32>,
	default_stamina: Option<u32>,
	default_satiety: Option<u32>,
	default_thirst: Option<u32>,
	default_mood: Option<u32>,
	default_health: Option<u32>,
	default_affinity: Option<u32>,
	default_experience: Option<u32>,
	default_level: Option<u32>,
}

impl PanelDebugConfigPartial {
	fn merge_into(self, mut base: PanelDebugConfig) -> PanelDebugConfig {
		if let Some(value) = self.basic_stat_max {
			base.basic_stat_max = value;
		}
		if let Some(value) = self.experience_max {
			base.experience_max = value;
		}
		if let Some(value) = self.level_max {
			base.level_max = value;
		}
		if let Some(value) = self.default_stamina {
			base.default_stamina = value;
		}
		if let Some(value) = self.default_satiety {
			base.default_satiety = value;
		}
		if let Some(value) = self.default_thirst {
			base.default_thirst = value;
		}
		if let Some(value) = self.default_mood {
			base.default_mood = value;
		}
		if let Some(value) = self.default_health {
			base.default_health = value;
		}
		if let Some(value) = self.default_affinity {
			base.default_affinity = value;
		}
		if let Some(value) = self.default_experience {
			base.default_experience = value;
		}
		if let Some(value) = self.default_level {
			base.default_level = value;
		}

		base
	}
}

#[derive(Clone, Debug)]
pub struct AnimationPathConfig {
	pub assets_body_root: String,
	pub default_happy_idle_variants: Vec<String>,
	pub default_nomal_idle_root: String,
	pub default_poor_condition_idle_root: String,
	pub default_ill_idle_root: String,
	pub startup_root: String,
	pub raise_dynamic_root: String,
	pub raise_static_root: String,
	pub pinch_root: String,
	pub shutdown_root: String,
	pub touch_head_root: String,
	pub touch_body_root: String,
}

impl Default for AnimationPathConfig {
	fn default() -> Self {
		Self {
			assets_body_root: ASSETS_BODY_ROOT.to_string(),
			default_happy_idle_variants: DEFAULT_HAPPY_IDLE_VARIANTS
				.iter()
				.map(|value| (*value).to_string())
				.collect(),
			default_nomal_idle_root: DEFAULT_NOMAL_IDLE_ROOT.to_string(),
			default_poor_condition_idle_root: DEFAULT_POOR_CONDITION_IDLE_ROOT.to_string(),
			default_ill_idle_root: DEFAULT_ILL_IDLE_ROOT.to_string(),
			startup_root: STARTUP_ROOT.to_string(),
			raise_dynamic_root: RAISE_DYNAMIC_ROOT.to_string(),
			raise_static_root: RAISE_STATIC_ROOT.to_string(),
			pinch_root: PINCH_ROOT.to_string(),
			shutdown_root: SHUTDOWN_ROOT.to_string(),
			touch_head_root: TOUCH_HEAD_ROOT.to_string(),
			touch_body_root: TOUCH_BODY_ROOT.to_string(),
		}
	}
}

impl AnimationPathConfig {
	fn sanitized(mut self) -> Self {
		let defaults = AnimationPathConfig::default();

		if self.assets_body_root.trim().is_empty() {
			self.assets_body_root = defaults.assets_body_root;
		}
		if self.startup_root.trim().is_empty() {
			self.startup_root = defaults.startup_root;
		}
		if self.raise_dynamic_root.trim().is_empty() {
			self.raise_dynamic_root = defaults.raise_dynamic_root;
		}
		if self.raise_static_root.trim().is_empty() {
			self.raise_static_root = defaults.raise_static_root;
		}
		if self.pinch_root.trim().is_empty() {
			self.pinch_root = defaults.pinch_root;
		}
		if self.shutdown_root.trim().is_empty() {
			self.shutdown_root = defaults.shutdown_root;
		}
		if self.touch_head_root.trim().is_empty() {
			self.touch_head_root = defaults.touch_head_root;
		}
		if self.touch_body_root.trim().is_empty() {
			self.touch_body_root = defaults.touch_body_root;
		}
		if self.default_nomal_idle_root.trim().is_empty() {
			self.default_nomal_idle_root = defaults.default_nomal_idle_root;
		}
		if self.default_poor_condition_idle_root.trim().is_empty() {
			self.default_poor_condition_idle_root = defaults.default_poor_condition_idle_root;
		}
		if self.default_ill_idle_root.trim().is_empty() {
			self.default_ill_idle_root = defaults.default_ill_idle_root;
		}
		self.default_happy_idle_variants = self
			.default_happy_idle_variants
			.into_iter()
			.map(|value| value.trim().to_string())
			.filter(|value| !value.is_empty())
			.collect();
		if self.default_happy_idle_variants.is_empty() {
			self.default_happy_idle_variants = defaults.default_happy_idle_variants;
		}

		self
	}
}

#[derive(Debug, Default, Deserialize)]
struct AnimationPathConfigPartial {
	assets_body_root: Option<String>,
	default_happy_idle_variants: Option<Vec<String>>,
	default_nomal_idle_root: Option<String>,
	default_poor_condition_idle_root: Option<String>,
	default_ill_idle_root: Option<String>,
	startup_root: Option<String>,
	raise_dynamic_root: Option<String>,
	raise_static_root: Option<String>,
	pinch_root: Option<String>,
	shutdown_root: Option<String>,
	touch_head_root: Option<String>,
	touch_body_root: Option<String>,
}

impl AnimationPathConfigPartial {
	fn merge_into(self, mut base: AnimationPathConfig) -> AnimationPathConfig {
		if let Some(value) = self.assets_body_root {
			base.assets_body_root = value;
		}
		if let Some(value) = self.default_happy_idle_variants {
			base.default_happy_idle_variants = value;
		}
		if let Some(value) = self.default_nomal_idle_root {
			base.default_nomal_idle_root = value;
		}
		if let Some(value) = self.default_poor_condition_idle_root {
			base.default_poor_condition_idle_root = value;
		}
		if let Some(value) = self.default_ill_idle_root {
			base.default_ill_idle_root = value;
		}
		if let Some(value) = self.startup_root {
			base.startup_root = value;
		}
		if let Some(value) = self.raise_dynamic_root {
			base.raise_dynamic_root = value;
		}
		if let Some(value) = self.raise_static_root {
			base.raise_static_root = value;
		}
		if let Some(value) = self.pinch_root {
			base.pinch_root = value;
		}
		if let Some(value) = self.shutdown_root {
			base.shutdown_root = value;
		}
		if let Some(value) = self.touch_head_root {
			base.touch_head_root = value;
		}
		if let Some(value) = self.touch_body_root {
			base.touch_body_root = value;
		}
		base
	}
}

pub fn runtime_config_path() -> PathBuf {
	PathBuf::from(RUNTIME_CONFIG_FILE)
}

fn load_file_config() -> Option<FileConfig> {
	let path = runtime_config_path();
	let content = match fs::read_to_string(&path) {
		Ok(content) => content,
		Err(err) => {
			if err.kind() != std::io::ErrorKind::NotFound {
				eprintln!("读取配置文件失败（{}）：{}", path.display(), err);
			}
			return None;
		}
	};

	match toml::from_str::<FileConfig>(&content) {
		Ok(file_config) => Some(file_config),
		Err(err) => {
			eprintln!("解析配置文件失败（{}）：{}", path.display(), err);
			None
		}
	}
}

pub fn load_panel_debug_config() -> PanelDebugConfig {
	let default_config = PanelDebugConfig::default();
	load_file_config()
		.and_then(|file_config| file_config.panel)
		.unwrap_or_default()
		.merge_into(default_config)
		.sanitized()
}

pub fn load_animation_path_config() -> AnimationPathConfig {
	let default_config = AnimationPathConfig::default();
	load_file_config()
		.and_then(|file_config| file_config.animation)
		.unwrap_or_default()
		.merge_into(default_config)
		.sanitized()
}

pub fn start_panel_config_watcher<F>(on_change: F) -> anyhow::Result<()>
where
	F: Fn() + Send + Sync + 'static,
{
	let config_path = runtime_config_path();
	let watch_target = config_path
		.parent()
		.unwrap_or_else(|| Path::new("."))
		.to_path_buf();
	let config_file_name = config_path
		.file_name()
		.map(|name| name.to_os_string())
		.unwrap_or_else(|| RUNTIME_CONFIG_FILE.into());

	let mut watcher = RecommendedWatcher::new(
		move |result: Result<Event, notify::Error>| {
			let event = match result {
				Ok(event) => event,
				Err(err) => {
					eprintln!("配置监听错误：{}", err);
					return;
				}
			};

			if !matches!(
				event.kind,
				EventKind::Create(_)
					| EventKind::Modify(_)
					| EventKind::Remove(_)
					| EventKind::Any
			) {
				return;
			}

			let is_target = event.paths.is_empty()
				|| event
					.paths
					.iter()
					.any(|path| path.file_name() == Some(config_file_name.as_os_str()));
			if is_target {
				on_change();
			}
		},
		notify::Config::default(),
	)
	.with_context(|| "创建配置文件监听器失败")?;

	watcher
		.watch(&watch_target, RecursiveMode::NonRecursive)
		.with_context(|| format!("监听配置目录失败：{}", watch_target.display()))?;

	let mut watchers = CONFIG_WATCHERS
		.lock()
		.map_err(|_| anyhow::anyhow!("配置监听器存储锁被污染"))?;
	watchers.push(watcher);

	Ok(())
}