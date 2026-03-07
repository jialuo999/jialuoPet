// ===== 依赖导入 =====
use std::cell::{Ref, RefCell};
use std::rc::Rc;

use crate::config::PanelDebugConfig;
use rand::{random, Rng};

use super::food::ItemDef;
use super::leveling::{add_exp, PetLevelState};
use super::model::{InteractType, PetMode, PetRuntimeState, PetStats};

// ===== 系统参数常量 =====
const LOGIC_INTERVAL_MIN_SECS: f64 = 5.0;
const LOGIC_INTERVAL_MAX_SECS: f64 = 60.0;
const REAL_SECS_PER_GAME_MIN: f64 = 300.0;

const HEALTH_MAX: f64 = 100.0;

const INTERACT_MIN_STRENGTH_REQUIRED: f64 = 10.0;
const INTERACT_STRENGTH_COST: f64 = 2.0;
const INTERACT_FEELING_GAIN: f64 = 1.0;
const WORK_BASE_FOOD_COST_PER_GAME_MIN: f64 = 1.0;
const WORK_BASE_DRINK_COST_PER_GAME_MIN: f64 = 1.0;
const WORK_BASE_OUTPUT_PER_GAME_MIN: f64 = 100.0;

// ===== 面板显示上限 =====
#[derive(Debug, Clone, Copy)]
struct PanelLimits {
    basic_stat_max: f64,
    experience_max: f64,
}

impl Default for PanelLimits {
    fn default() -> Self {
        Self {
            basic_stat_max: 100.0,
            experience_max: 100.0,
        }
    }
}

// ===== 宠物数值服务 =====
#[derive(Debug, Clone)]
pub struct PetStatsService {
    stats: Rc<RefCell<PetStats>>,
    limits: Rc<RefCell<PanelLimits>>,
    runtime_state: Rc<RefCell<PetRuntimeState>>,
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
            runtime_state: Rc::new(RefCell::new(PetRuntimeState::Nomal)),
            secs_since_last_interact: Rc::new(RefCell::new(0.0)),
            logic_interval_secs: clamp_logic_interval(logic_interval_secs),
        }
    }

    #[allow(dead_code)]
    pub fn from_shared(stats: Rc<RefCell<PetStats>>, logic_interval_secs: f64) -> Self {
        Self {
            stats,
            limits: Rc::new(RefCell::new(PanelLimits::default())),
            runtime_state: Rc::new(RefCell::new(PetRuntimeState::Nomal)),
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

    #[allow(dead_code)]
    pub fn set_runtime_state(&self, runtime_state: PetRuntimeState) {
        *self.runtime_state.borrow_mut() = runtime_state;
    }

    pub fn runtime_state(&self) -> PetRuntimeState {
        *self.runtime_state.borrow()
    }

    // 配置更新只调整面板显示上限，不覆盖运行中的角色数值。
    pub fn apply_panel_config(&self, panel_config: PanelDebugConfig) {
        {
            let mut limits = self.limits.borrow_mut();
            limits.basic_stat_max = panel_config.basic_stat_max as f64;
            limits.experience_max = panel_config.experience_max as f64;
        }
    }

    #[allow(dead_code)]
    pub fn basic_stat_max(&self) -> f64 {
        self.limits.borrow().basic_stat_max
    }

    #[allow(dead_code)]
    pub fn experience_max(&self) -> f64 {
        self.limits.borrow().experience_max
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

        let game_minutes = delta_secs / REAL_SECS_PER_GAME_MIN;

        {
            let mut secs_since_last_interact = self.secs_since_last_interact.borrow_mut();
            *secs_since_last_interact += delta_secs;
        }

        match self.runtime_state() {
            PetRuntimeState::Sleep => self.apply_sleep_tick(game_minutes),
            PetRuntimeState::Work => self.apply_work_tick(game_minutes),
            PetRuntimeState::Nomal => self.apply_nomal_tick(game_minutes),
        }

        self.apply_interaction_decay(game_minutes);
        self.apply_negative_penalties();
        self.apply_global_feeling_bonus(game_minutes);
        self.apply_global_drink_effect(game_minutes);

        let mut stats = self.stats.borrow_mut();
        clamp_stats(&mut stats);
        apply_level_up_if_needed(&mut stats);
        clamp_stats(&mut stats);
    }

    // 统一物品使用入口：所有商品效果都通过 effects 字段生效。
    pub fn on_use_item(&mut self, item: &ItemDef) -> bool {
        {
            let mut stats = self.stats.borrow_mut();
            if stats.money < item.price as u64 {
                return false;
            }
            stats.money -= item.price as u64;
        }

        self.apply_likability_gain(item.effects.likability);
        self.apply_feeling_gain(item.effects.mood);

        let mut stats = self.stats.borrow_mut();
        stats.strength_food += item.effects.satiety;
        stats.strength_drink += item.effects.thirst;
        stats.strength += item.effects.stamina;
        stats.health += item.effects.health;
        stats.exp += item.effects.exp;

        clamp_stats(&mut stats);
        apply_level_up_if_needed(&mut stats);
        clamp_stats(&mut stats);

        true
    }

    #[allow(dead_code)]
    pub fn on_feed(&mut self, item: &ItemDef) -> bool {
        self.on_use_item(item)
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

    fn apply_sleep_tick(&mut self, game_minutes: f64) {
        let mut stats = self.stats.borrow_mut();
        let low_food = stats.strength_max * 0.25;
        let high_food = stats.strength_max * 0.75;
        let high_drink = stats.strength_max * 0.75;

        stats.strength += game_minutes * 2.0;
        stats.strength_food += game_minutes * 1.0;
        stats.strength_drink += game_minutes * 1.0;
        stats.exp += game_minutes;

        if stats.strength_food <= low_food {
            stats.strength_food += game_minutes * 1.0;
        }
        if stats.strength_drink >= high_drink {
            stats.strength_drink += game_minutes * 1.0;
        }
        if stats.strength_food >= high_food {
            stats.health += game_minutes;
        }
        if stats.strength_drink >= high_drink {
            stats.health += game_minutes;
        }
    }

    fn apply_work_tick(&mut self, game_minutes: f64) {
        let mut stats = self.stats.borrow_mut();

        let needed_food = WORK_BASE_FOOD_COST_PER_GAME_MIN * game_minutes;
        let needed_drink = WORK_BASE_DRINK_COST_PER_GAME_MIN * game_minutes;
        let low_threshold = stats.strength_max * 0.25;
        let high_threshold = stats.strength_max * 0.60;

        let mut efficiency = 0.0;

        if stats.strength > stats.strength_max * 0.25 + needed_food * 0.3 + needed_drink * 0.3 {
            efficiency += 0.1;
            stats.strength -= (needed_food + needed_drink) * 0.3;
        }

        let food_cost = if stats.strength_food <= low_threshold {
            efficiency += 0.2;
            if stats.strength >= needed_food {
                efficiency += 0.1;
            }
            needed_food * 0.5
        } else {
            efficiency += 0.4;
            if stats.strength_food >= high_threshold {
                efficiency += 0.1;
            }
            needed_food
        };

        let drink_cost = if stats.strength_drink <= low_threshold {
            efficiency += 0.2;
            if stats.strength >= needed_drink {
                efficiency += 0.1;
            }
            needed_drink * 0.5
        } else {
            efficiency += 0.4;
            if stats.strength_drink >= high_threshold {
                efficiency += 0.1;
            }
            needed_drink
        };

        stats.strength_food -= food_cost;
        stats.strength_drink -= drink_cost;
        stats.health -= 2.0 * game_minutes;

        if stats.strength_food >= high_threshold || stats.strength_drink >= high_threshold {
            let bonus = rand::thread_rng().gen_range(1.0..=3.0);
            stats.health += bonus * game_minutes;
        }

        let base_output = WORK_BASE_OUTPUT_PER_GAME_MIN * game_minutes;
        let output = (base_output * (2.0 * efficiency - 0.5)).max(0.0);
        stats.exp += output;
        stats.money = stats.money.saturating_add(output.round() as u64);
    }

    fn apply_nomal_tick(&mut self, game_minutes: f64) {
        let mut stats = self.stats.borrow_mut();
        let half = stats.strength_max * 0.5;
        let quarter = stats.strength_max * 0.25;

        if stats.strength_food >= half {
            stats.strength_food -= game_minutes;
        } else if stats.strength_food <= quarter {
            stats.strength_food += 0.0;
        }

        if stats.strength_drink >= half {
            stats.strength_drink -= 2.0 * game_minutes;
            stats.strength += game_minutes;
        } else if stats.strength_drink <= quarter {
            stats.strength_drink += 0.0;
        }

        let random_health_drift = (random::<f64>() - 0.5) * game_minutes;
        stats.health += random_health_drift;
        stats.exp += game_minutes;
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
    let mut level_state = PetLevelState {
        level: stats.level.max(1),
        level_max: stats.level_stage,
        exp: stats.exp,
        likability_max: stats.likability_max,
        strength_max: stats.strength_max,
        feeling_max: stats.feeling_max,
    };

    add_exp(&mut level_state, 0.0);

    stats.level = level_state.level;
    stats.level_stage = level_state.level_max;
    stats.exp = level_state.exp;
    stats.likability_max = level_state.likability_max;
    stats.strength_max = level_state.strength_max;
    stats.feeling_max = level_state.feeling_max;
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

fn clamp_logic_interval(value: f64) -> f64 {
    value.clamp(LOGIC_INTERVAL_MIN_SECS, LOGIC_INTERVAL_MAX_SECS)
}

fn random_binary_f64() -> f64 {
    if random::<bool>() {
        1.0
    } else {
        0.0
    }
}
