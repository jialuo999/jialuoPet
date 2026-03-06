// ===== 物品类型定义 =====
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    Staple,
    Snack,
    Drink,
    Gift,
    Drug,
    Functional,
}

// ===== 通用效果字段 =====
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemEffects {
    pub satiety: f64,
    pub mood: f64,
    pub thirst: f64,
    pub health: f64,
    pub likability: f64,
    pub stamina: f64,
    pub exp: f64,
}

impl Default for ItemEffects {
    fn default() -> Self {
        Self {
            satiety: 0.0,
            mood: 0.0,
            thirst: 0.0,
            health: 0.0,
            likability: 0.0,
            stamina: 0.0,
            exp: 0.0,
        }
    }
}

// ===== 通用物品定义 =====
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct ItemDef {
    pub id: String,
    pub kind: ItemKind,
    pub price: u32,
    pub stack_limit: u32,
    pub effects: ItemEffects,
}
