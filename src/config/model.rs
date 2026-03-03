// ===== 依赖导入 =====
use serde::Deserialize;

use super::defaults::{
	ASSETS_BODY_ROOT, DEFAULT_HAPPY_IDLE_VARIANTS, DEFAULT_ILL_IDLE_ROOT,
	DEFAULT_NOMAL_IDLE_ROOT, DEFAULT_POOR_CONDITION_IDLE_ROOT, PANEL_BASIC_STAT_MAX,
	PANEL_DEFAULT_AFFINITY, PANEL_DEFAULT_EXPERIENCE, PANEL_DEFAULT_HEALTH,
	PANEL_DEFAULT_LEVEL, PANEL_DEFAULT_MOOD, PANEL_DEFAULT_SATIETY, PANEL_DEFAULT_STAMINA,
	PANEL_DEFAULT_THIRST, PANEL_EXPERIENCE_MAX, PINCH_ROOT,
	RAISE_DYNAMIC_ROOT, RAISE_STATIC_ROOT, SHUTDOWN_ROOT, STARTUP_ROOT, TOUCH_BODY_ROOT,
	TOUCH_HEAD_ROOT,
};

// ===== 面板配置结构 =====
#[derive(Clone, Debug)]
pub struct PanelDebugConfig {
	pub basic_stat_max: u32,
	pub experience_max: u32,
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
	// 归一化配置，确保上下限合法
	pub(crate) fn sanitized(mut self) -> Self {
		self.basic_stat_max = self.basic_stat_max.max(1);
		self.experience_max = self.experience_max.max(1);

		self.default_stamina = self.default_stamina.min(self.basic_stat_max);
		self.default_satiety = self.default_satiety.min(self.basic_stat_max);
		self.default_thirst = self.default_thirst.min(self.basic_stat_max);
		self.default_mood = self.default_mood.min(self.basic_stat_max);
		self.default_health = self.default_health.min(self.basic_stat_max);
		self.default_affinity = self.default_affinity.min(self.basic_stat_max);
		self.default_experience = self.default_experience.min(self.experience_max);
		self.default_level = self.default_level.max(1);

		self
	}
}

// ===== 配置文件反序列化结构 =====
#[derive(Debug, Default, Deserialize)]
pub(crate) struct FileConfig {
	pub(crate) panel: Option<PanelDebugConfigPartial>,
	pub(crate) animation: Option<AnimationPathConfigPartial>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct PanelDebugConfigPartial {
	basic_stat_max: Option<u32>,
	experience_max: Option<u32>,
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
	// 将部分配置覆盖到完整配置
	pub(crate) fn merge_into(self, mut base: PanelDebugConfig) -> PanelDebugConfig {
		if let Some(value) = self.basic_stat_max {
			base.basic_stat_max = value;
		}
		if let Some(value) = self.experience_max {
			base.experience_max = value;
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

// ===== 动画路径配置结构 =====
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
	// 归一化配置，空字符串回退到默认值
	pub(crate) fn sanitized(mut self) -> Self {
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
pub(crate) struct AnimationPathConfigPartial {
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
	// 将部分配置覆盖到完整配置
	pub(crate) fn merge_into(self, mut base: AnimationPathConfig) -> AnimationPathConfig {
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
