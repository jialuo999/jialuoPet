use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ===== 宠物状态分类 =====
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetMode {
    Happy,
    Nomal,
    PoorCondition,
    Ill,
}

// ===== 用户交互类型 =====
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractType {
    TouchHead,
    TouchBody,
    Pinch,
    Study,
    Work,
    Play,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetRuntimeState {
    Sleep,
    Study,
    Work,
    Nomal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StudyMode {
    Book,
    Paint,
    Research,
}

impl StudyMode {
    pub fn from_label(label: &str) -> Self {
        match label {
            "画画" => Self::Paint,
            "研究" => Self::Research,
            _ => Self::Book,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Book => "看书",
            Self::Paint => "画画",
            Self::Research => "研究",
        }
    }
}

// ===== 背包物品条目 =====
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InventoryItem {
    pub item_id: String,
    pub count: u32,
}

// ===== 背包 =====
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct Inventory {
    pub items: HashMap<String, u32>, // item_id -> count
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            items: HashMap::new(),
        }
    }
}

impl Inventory {
    pub fn add(&mut self, item_id: &str, count: u32) {
        let entry = self.items.entry(item_id.to_string()).or_insert(0);
        *entry = entry.saturating_add(count);
    }

    pub fn remove(&mut self, item_id: &str, count: u32) -> bool {
        if let Some(&current) = self.items.get(item_id) {
            if current >= count {
                if current == count {
                    self.items.remove(item_id);
                } else {
                    self.items.insert(item_id.to_string(), current - count);
                }
                return true;
            }
        }
        false
    }

    pub fn get(&self, item_id: &str) -> u32 {
        self.items.get(item_id).copied().unwrap_or(0)
    }

    pub fn list_items(&self) -> Vec<(String, u32)> {
        self.items
            .iter()
            .map(|(id, count)| (id.clone(), *count))
            .collect()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.items.clear();
    }
}

// ===== 宠物核心数值模型 =====
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct PetStats {
    pub health: f64,
    pub feeling: f64,
    pub feeling_max: f64,
    pub likability_max: f64,
    pub strength: f64,
    pub strength_max: f64,
    pub strength_food: f64,
    pub strength_drink: f64,
    pub likability: f64,
    pub level: u32,
    pub level_stage: u32,
    pub exp: f64,
    pub money: u64,
    pub inventory: Inventory,
}

impl Default for PetStats {
    fn default() -> Self {
        let level = 1;
        let level_stage = 0;
        let feeling_max = Self::feeling_max_for_level(level, level_stage);
        let likability_max = Self::initial_likability_max();
        let strength_max = Self::strength_max_for_level(level, level_stage);

        Self {
            health: 100.0,
            feeling: 100.0,
            feeling_max,
            likability_max,
            strength: strength_max,
            strength_max,
            strength_food: 100.0,
            strength_drink: 100.0,
            likability: 0.0,
            level,
            level_stage,
            exp: 0.0,
            money: 1000,
            inventory: Inventory::default(),
        }
    }
}

impl PetStats {
	// 计算“真实健康阈值”（受心情与好感加成影响）
    pub fn real_health_threshold(&self) -> f64 {
        let safe_feeling_max = self.feeling_max.max(f64::EPSILON);
        let felps = self.feeling / safe_feeling_max;

        let feeling_bonus = if felps >= 0.8 { 12.0 } else { 0.0 };
        let likability_bonus = if self.likability >= 80.0 {
            12.0
        } else if self.likability >= 40.0 {
            6.0
        } else {
            0.0
        };

        (60.0_f64 - feeling_bonus - likability_bonus).max(0.0_f64)
    }

	// 根据当前数值推导模式
    pub fn cal_mode(&self) -> PetMode {
        let realhel = self.real_health_threshold();

        if self.health <= realhel / 2.0 {
            return PetMode::Ill;
        }
        if self.health <= realhel {
            return PetMode::PoorCondition;
        }

        let safe_feeling_max = self.feeling_max.max(f64::EPSILON);
        let felps = self.feeling / safe_feeling_max;

        let likability_bonus = if self.likability >= 80.0 {
            0.20
        } else if self.likability >= 40.0 {
            0.10
        } else {
            0.0
        };
        let realfel = 0.90 - likability_bonus;

        if felps >= realfel {
            PetMode::Happy
        } else if felps <= realfel / 2.0 {
            PetMode::PoorCondition
        } else {
            PetMode::Nomal
        }
    }

	// 升级所需经验
    #[allow(dead_code)]
    pub fn level_up_exp_needed(&self) -> f64 {
        200.0 * self.level as f64 - 100.0
    }

	// 随等级增长的上限曲线
    pub fn feeling_max_for_level(level: u32, level_stage: u32) -> f64 {
        let level_f = level as f64;
        let level_stage_f = level_stage as f64;
        100.0 + ((level_f * (1.0 + level_stage_f)).powf(0.75) * 2.0).floor()
    }

    pub fn strength_max_for_level(level: u32, level_stage: u32) -> f64 {
        let level_f = level as f64;
        let level_stage_f = level_stage as f64;
        100.0 + ((level_f * (1.0 + level_stage_f)).powf(0.75) * 4.0).floor()
    }

    pub fn initial_likability_max() -> f64 {
        100.0
    }
}
