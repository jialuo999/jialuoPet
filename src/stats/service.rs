// ===== 依赖导入 =====
use std::cell::{Ref, RefCell};
use std::rc::Rc;

use crate::config::PanelDebugConfig;
use rand::random;

use super::food::FoodItem;
use super::model::{InteractType, PetMode, PetStats};

// ===== 系统参数常量 =====
const LOGIC_INTERVAL_MIN_SECS: f64 = 5.0;
const LOGIC_INTERVAL_MAX_SECS: f64 = 60.0;

const HEALTH_MAX: f64 = 100.0;

const DECAY_BALANCE_STRENGTH: f64 = 0.8;
const DECAY_BALANCE_FOOD_DRINK: f64 = 1.0;
const INTERACT_MIN_STRENGTH_REQUIRED: f64 = 10.0;
const INTERACT_STRENGTH_COST: f64 = 2.0;
const INTERACT_FEELING_GAIN: f64 = 1.0;

// ===== 面板显示上限 =====
#[derive(Debug, Clone, Copy)]
struct PanelLimits {
    basic_stat_max: f64,
    experience_max: f64,
    level_max: f64,
}

impl Default for PanelLimits {
    fn default() -> Self {
        Self {
            basic_stat_max: 100.0,
            experience_max: 100.0,
            level_max: 100.0,
        }
    }
}

// ===== 宠物数值服务 =====
#[derive(Debug, Clone)]
pub struct PetStatsService {
    stats: Rc<RefCell<PetStats>>,
    limits: Rc<RefCell<PanelLimits>>,
    secs_since_last_interact: Rc<RefCell<f64>>,
    logic_interval_secs: f64,
}

impl Default for PetStatsService {
    fn default() -> Self {
        Self::new(PetStats::default(), LOGIC_INTERVAL_MIN_SECS)
    }
}

impl PetStatsService {
	// 构造服务并限制逻辑间隔范围
    pub fn new(initial_stats: PetStats, logic_interval_secs: f64) -> Self {
        Self {
            stats: Rc::new(RefCell::new(initial_stats)),
            limits: Rc::new(RefCell::new(PanelLimits::default())),
            secs_since_last_interact: Rc::new(RefCell::new(0.0)),
            logic_interval_secs: clamp_logic_interval(logic_interval_secs),
        }
    }

    pub fn from_panel_config(panel_config: PanelDebugConfig, logic_interval_secs: f64) -> Self {
        let service = Self::new(panel_config_to_stats(&panel_config), logic_interval_secs);
        service.apply_panel_config(panel_config);
        service
    }

    #[allow(dead_code)]
    pub fn from_shared(stats: Rc<RefCell<PetStats>>, logic_interval_secs: f64) -> Self {
        Self {
            stats,
            limits: Rc::new(RefCell::new(PanelLimits::default())),
            secs_since_last_interact: Rc::new(RefCell::new(0.0)),
            logic_interval_secs: clamp_logic_interval(logic_interval_secs),
        }
    }

    #[allow(dead_code)]
    pub fn stats(&self) -> Ref<'_, PetStats> {
        self.stats.borrow()
    }

    pub fn get_stats(&self) -> PetStats {
        self.stats.borrow().clone()
    }

    pub fn replace_stats(&self, next_stats: PetStats) {
        *self.stats.borrow_mut() = next_stats;
    }

	// 每次配置更新都重建数值与显示上限
    pub fn apply_panel_config(&self, panel_config: PanelDebugConfig) {
        {
            let mut limits = self.limits.borrow_mut();
            limits.basic_stat_max = panel_config.basic_stat_max as f64;
            limits.experience_max = panel_config.experience_max as f64;
            limits.level_max = panel_config.level_max as f64;
        }
        self.replace_stats(panel_config_to_stats(&panel_config));
    }

    #[allow(dead_code)]
    pub fn basic_stat_max(&self) -> f64 {
        self.limits.borrow().basic_stat_max
    }

    pub fn experience_max(&self) -> f64 {
        self.limits.borrow().experience_max
    }

    pub fn level_max(&self) -> f64 {
        self.limits.borrow().level_max
    }

    #[allow(dead_code)]
    pub fn shared_stats(&self) -> Rc<RefCell<PetStats>> {
        self.stats.clone()
    }

    #[allow(dead_code)]
    pub fn logic_interval_secs(&self) -> f64 {
        self.logic_interval_secs
    }

    #[allow(dead_code)]
    pub fn set_logic_interval_secs(&mut self, logic_interval_secs: f64) {
        self.logic_interval_secs = clamp_logic_interval(logic_interval_secs);
    }

	// 逻辑 tick：处理自然衰减与升级
    pub fn on_tick(&mut self, delta_secs: f64) {
        if delta_secs <= 0.0 {
            return;
        }

        let scale = delta_secs / self.logic_interval_secs;

        {
            let mut secs_since_last_interact = self.secs_since_last_interact.borrow_mut();
            *secs_since_last_interact += delta_secs;
        }

        {
            let mut stats = self.stats.borrow_mut();
            stats.strength_food -= DECAY_BALANCE_FOOD_DRINK * scale;
            stats.strength_drink -= DECAY_BALANCE_FOOD_DRINK * scale;
            stats.strength -= DECAY_BALANCE_STRENGTH * scale;
        }

        self.apply_interaction_decay(scale);
        self.apply_negative_penalties();
        self.apply_global_feeling_bonus(scale);
        self.apply_global_drink_effect(scale);

        let mut stats = self.stats.borrow_mut();
        clamp_stats(&mut stats);
        apply_level_up_if_needed(&mut stats);
        clamp_stats(&mut stats);
    }

	// 投喂：恢复基础属性并增加好感
    #[allow(dead_code)]
    pub fn on_feed(&mut self, food: &FoodItem) {
        self.apply_likability_gain(food.likability);

        let bonus = {
            let stats = self.stats.borrow();
            likability_bonus(stats.likability)
        };
        let recover_factor = 1.0 + bonus;

        self.apply_feeling_gain(food.feeling * recover_factor);

        let mut stats = self.stats.borrow_mut();
        stats.strength_food += food.strength_food * recover_factor;
        stats.strength_drink += food.strength_drink * recover_factor;

        clamp_stats(&mut stats);
    }

	// 互动：消耗体力并变化心情/经验
    #[allow(dead_code)]
    pub fn on_interact(&mut self, _interact_type: InteractType) -> bool {
        let (can_animate, should_apply_effect) = {
            let stats = self.stats.borrow();
            let has_enough_strength = stats.strength >= INTERACT_MIN_STRENGTH_REQUIRED;
            let feeling_not_full = stats.feeling < stats.feeling_max;
            (has_enough_strength, has_enough_strength && feeling_not_full)
        };

        if !can_animate {
            return false;
        }

        *self.secs_since_last_interact.borrow_mut() = 0.0;

        if !should_apply_effect {
            return true;
        }

        {
            let mut stats = self.stats.borrow_mut();
            stats.strength =
                (stats.strength - INTERACT_STRENGTH_COST).clamp(0.0, stats.strength_max);
            let level_f = stats.level as f64;
            stats.exp += 1.0 * level_f;
        }

        self.apply_feeling_gain(INTERACT_FEELING_GAIN);

        let mut stats = self.stats.borrow_mut();

        clamp_stats(&mut stats);
        apply_level_up_if_needed(&mut stats);
        clamp_stats(&mut stats);

        true
    }

	// 心情变化统一入口（并联动好感）
    fn apply_feeling_gain(&mut self, feeling_gain: f64) {
        let raw_feeling = {
            let stats = self.stats.borrow();
            stats.feeling + feeling_gain
        };

        self.apply_likability_gain(feeling_gain);

        let mut stats = self.stats.borrow_mut();
        stats.feeling = raw_feeling.clamp(0.0, stats.feeling_max);
    }

	// 好感变化统一入口（溢出转为健康）
    fn apply_likability_gain(&mut self, delta: f64) {
        let mut stats = self.stats.borrow_mut();
        let new_val = stats.likability + delta;
        if new_val > stats.likability_max {
            let overflow = new_val - stats.likability_max;
            stats.likability = stats.likability_max;
            stats.health = (stats.health + overflow).min(100.0);
        } else {
            stats.likability = new_val.clamp(0.0, stats.likability_max);
        }
    }

    fn apply_interaction_decay(&mut self, time_scale: f64) {
        let idle_minutes = *self.secs_since_last_interact.borrow() / 60.0;
        if idle_minutes < 1.0 {
            return;
        }

        let mut stats = self.stats.borrow_mut();
        let feeling_decay = (idle_minutes.sqrt() * time_scale / 4.0).min(stats.feeling_max / 800.0);
        stats.feeling -= feeling_decay;
    }

    fn apply_negative_penalties(&mut self) {
        let mut stats = self.stats.borrow_mut();

        if stats.strength_food <= 0.0 {
            stats.health += stats.strength_food;
            stats.strength_food = 0.0;
        }

        if stats.strength_drink <= 0.0 {
            stats.health += stats.strength_drink;
            stats.strength_drink = 0.0;
        }

        if stats.feeling <= 0.0 {
            let feeling_penalty = stats.feeling / 2.0;
            stats.health += feeling_penalty;
            stats.likability += feeling_penalty;
            stats.feeling = 0.0;
        }
    }

    fn apply_global_feeling_bonus(&mut self, time_scale: f64) {
        let mut stats = self.stats.borrow_mut();

        if stats.feeling >= stats.feeling_max * 0.75 {
            if stats.feeling >= stats.feeling_max * 0.90 {
                stats.likability += time_scale;
            }
            stats.exp += time_scale * 2.0;
            stats.health += time_scale;
        } else if stats.feeling <= 25.0 {
            stats.likability -= time_scale;
            stats.exp -= time_scale;
        }
    }

    fn apply_global_drink_effect(&mut self, time_scale: f64) {
        let mut stats = self.stats.borrow_mut();
        let quarter = stats.strength_max * 0.25;
        let three_quarters = stats.strength_max * 0.75;

        if stats.strength_drink <= quarter {
            stats.health -= random_binary_f64() * time_scale;
            stats.exp -= time_scale;
        } else if stats.strength_drink >= three_quarters {
            stats.health += random_binary_f64() * time_scale;
        }
    }

    pub fn cal_mode(&self) -> PetMode {
        self.stats.borrow().cal_mode()
    }
}

// ===== 私有辅助函数 =====
fn apply_level_up_if_needed(stats: &mut PetStats) {
    loop {
        let needed = stats.level_up_exp_needed();
        if stats.exp < needed {
            break;
        }

        stats.exp -= needed;
        stats.level = stats.level.saturating_add(1);

        let stage_gate = 1000_u32.saturating_add(stats.level_stage.saturating_mul(100));
        if stats.level > stage_gate {
            stats.level_stage = stats.level_stage.saturating_add(1);
            stats.level = 100_u32.saturating_mul(stats.level_stage).max(1);
        }

        stats.feeling_max = PetStats::feeling_max_for_level(stats.level, stats.level_stage);
        stats.likability_max = PetStats::likability_max_for_level(stats.level);
        stats.strength_max = PetStats::strength_max_for_level(stats.level, stats.level_stage);
    }
}

fn clamp_stats(stats: &mut PetStats) {
    let strength_max = stats.strength_max.max(0.0);
    let feeling_max = stats.feeling_max.max(0.0);
    let likability_max = stats.likability_max.max(0.0);

    if stats.likability > likability_max {
        let overflow = stats.likability - likability_max;
        stats.likability = likability_max;
        stats.health += overflow;
    } else {
        stats.likability = stats.likability.max(0.0);
    }

    stats.health = stats.health.clamp(0.0, HEALTH_MAX);
    stats.feeling = stats.feeling.clamp(0.0, feeling_max);
    stats.strength = stats.strength.clamp(0.0, strength_max);
    stats.strength_food = stats.strength_food.clamp(0.0, strength_max);
    stats.strength_drink = stats.strength_drink.clamp(0.0, strength_max);
    stats.likability = stats.likability.clamp(0.0, likability_max);
    stats.exp = stats.exp.max(0.0);
}

fn panel_config_to_stats(panel_config: &PanelDebugConfig) -> PetStats {
    let requested_level = panel_config.default_level.max(1);
    let (level, level_stage) = normalize_level_and_stage(requested_level);
    let feeling_max = PetStats::feeling_max_for_level(level, level_stage);
    let strength_max = PetStats::strength_max_for_level(level, level_stage);
    let likability_max = PetStats::likability_max_for_level(level);

    PetStats {
        health: (panel_config.default_health as f64).clamp(0.0, HEALTH_MAX),
        feeling: (panel_config.default_mood as f64).clamp(0.0, feeling_max),
        feeling_max,
        likability_max,
        strength: (panel_config.default_stamina as f64).clamp(0.0, strength_max),
        strength_max,
        strength_food: (panel_config.default_satiety as f64).clamp(0.0, strength_max),
        strength_drink: (panel_config.default_thirst as f64).clamp(0.0, strength_max),
        likability: (panel_config.default_affinity as f64).clamp(0.0, likability_max),
        level,
        level_stage,
        exp: panel_config.default_experience as f64,
    }
}

#[allow(dead_code)]
fn likability_bonus(likability: f64) -> f64 {
    if likability >= 80.0 {
        0.20
    } else if likability >= 40.0 {
        0.10
    } else {
        0.0
    }
}

fn clamp_logic_interval(value: f64) -> f64 {
    value.clamp(LOGIC_INTERVAL_MIN_SECS, LOGIC_INTERVAL_MAX_SECS)
}

fn normalize_level_and_stage(level: u32) -> (u32, u32) {
    let mut current_level = level.max(1);
    let mut level_stage = 0_u32;

    loop {
        let stage_gate = 1000_u32.saturating_add(level_stage.saturating_mul(100));
        if current_level <= stage_gate {
            break;
        }

        level_stage = level_stage.saturating_add(1);
        current_level = 100_u32.saturating_mul(level_stage).max(1);
    }

    (current_level, level_stage)
}

fn random_binary_f64() -> f64 {
    if random::<bool>() {
        1.0
    } else {
        0.0
    }
}
