use std::cell::{Ref, RefCell};
use std::rc::Rc;

use crate::config::PanelDebugConfig;

use super::model::{PetMode, PetStats};

const LOGIC_INTERVAL_MIN_SECS: f64 = 5.0;
const LOGIC_INTERVAL_MAX_SECS: f64 = 60.0;

const DECAY_BASE: f64 = 1.0;
const HEALTH_MAX: f64 = 100.0;

const DECAY_BALANCE_FOOD_DRINK: f64 = 1.0;
const DECAY_BALANCE_STRENGTH: f64 = 0.8;
const DECAY_BALANCE_FEELING: f64 = 0.5;
const DECAY_BALANCE_HEALTH: f64 = 0.3;

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

#[derive(Debug, Clone)]
pub struct PetStatsService {
    stats: Rc<RefCell<PetStats>>,
    limits: Rc<RefCell<PanelLimits>>,
    logic_interval_secs: f64,
}

impl Default for PetStatsService {
    fn default() -> Self {
        Self::new(PetStats::default(), LOGIC_INTERVAL_MIN_SECS)
    }
}

impl PetStatsService {
    pub fn new(initial_stats: PetStats, logic_interval_secs: f64) -> Self {
        Self {
            stats: Rc::new(RefCell::new(initial_stats)),
            limits: Rc::new(RefCell::new(PanelLimits::default())),
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

    pub fn apply_panel_config(&self, panel_config: PanelDebugConfig) {
        {
            let mut limits = self.limits.borrow_mut();
            limits.basic_stat_max = panel_config.basic_stat_max as f64;
            limits.experience_max = panel_config.experience_max as f64;
            limits.level_max = panel_config.level_max as f64;
        }
        self.replace_stats(panel_config_to_stats(&panel_config));
    }

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

    pub fn on_tick(&mut self, delta_secs: f64) {
        if delta_secs <= 0.0 {
            return;
        }

        let scale = (delta_secs / self.logic_interval_secs) * DECAY_BASE;

        let basic_stat_max = self.basic_stat_max();
        let mut stats = self.stats.borrow_mut();

        stats.strength_food -= DECAY_BALANCE_FOOD_DRINK * scale;
        stats.strength_drink -= DECAY_BALANCE_FOOD_DRINK * scale;
        stats.strength -= DECAY_BALANCE_STRENGTH * scale;

        if stats.strength < 30.0 {
            stats.feeling -= DECAY_BALANCE_FEELING * scale;
        }

        if stats.feeling < 20.0 && stats.strength_food < 10.0 && stats.strength_drink < 10.0 {
            stats.health -= DECAY_BALANCE_HEALTH * scale;
        }

        clamp_stats(&mut stats, basic_stat_max);
        apply_level_up_if_needed(&mut stats);
        clamp_stats(&mut stats, basic_stat_max);
    }

    #[allow(dead_code)]
    pub fn on_feed(&mut self, food_strength: f64, food_drink: f64, food_feeling: f64) {
        let basic_stat_max = self.basic_stat_max();
        let mut stats = self.stats.borrow_mut();
        let bonus = likability_bonus(stats.likability);
        let recover_factor = 1.0 + bonus;

        stats.strength_food += food_strength * recover_factor;
        stats.strength_drink += food_drink * recover_factor;
        stats.feeling += food_feeling * recover_factor;

        clamp_stats(&mut stats, basic_stat_max);
    }

    #[allow(dead_code)]
    pub fn on_interact(&mut self) {
        let basic_stat_max = self.basic_stat_max();
        let mut stats = self.stats.borrow_mut();

        let level_f = stats.level as f64;
        stats.feeling += 5.0 * (1.0 + level_f / 10.0);
        stats.exp += 1.0 * level_f;

        clamp_stats(&mut stats, basic_stat_max);
        apply_level_up_if_needed(&mut stats);
        clamp_stats(&mut stats, basic_stat_max);
    }

    pub fn cal_mode(&self) -> PetMode {
        self.stats.borrow().cal_mode()
    }
}

fn apply_level_up_if_needed(stats: &mut PetStats) {
    loop {
        let needed = stats.level_up_exp_needed();
        if stats.exp < needed {
            break;
        }

        stats.exp -= needed;
        stats.level = stats.level.saturating_add(1);
        stats.feeling_max = PetStats::feeling_max_for_level(stats.level);
        stats.strength_max = PetStats::strength_max_for_level(stats.level);
    }
}

fn clamp_stats(stats: &mut PetStats, basic_stat_max: f64) {
    let base_max = basic_stat_max.max(1.0);

    stats.health = stats.health.clamp(0.0, HEALTH_MAX);
    stats.feeling = stats.feeling.clamp(0.0, stats.feeling_max.max(0.0));
    stats.strength = stats.strength.clamp(0.0, stats.strength_max.max(0.0));
    stats.strength_food = stats.strength_food.clamp(0.0, base_max);
    stats.strength_drink = stats.strength_drink.clamp(0.0, base_max);
    stats.likability = stats.likability.clamp(0.0, base_max);
    stats.exp = stats.exp.max(0.0);
}

fn panel_config_to_stats(panel_config: &PanelDebugConfig) -> PetStats {
    let level = panel_config.default_level.max(1);
    let basic_stat_max = panel_config.basic_stat_max as f64;

    PetStats {
        health: panel_config.default_health as f64,
        feeling: panel_config.default_mood as f64,
        feeling_max: basic_stat_max,
        strength: panel_config.default_stamina as f64,
        strength_max: basic_stat_max,
        strength_food: panel_config.default_satiety as f64,
        strength_drink: panel_config.default_thirst as f64,
        likability: panel_config.default_affinity as f64,
        level,
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
