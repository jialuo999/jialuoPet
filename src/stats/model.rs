#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetMode {
    Happy,
    Nomal,
    PoorCondition,
    Ill,
}

#[derive(Debug, Clone)]
pub struct PetStats {
    pub health: f64,
    pub feeling: f64,
    pub feeling_max: f64,
    pub strength: f64,
    pub strength_max: f64,
    pub strength_food: f64,
    pub strength_drink: f64,
    pub likability: f64,
    pub level: u32,
    pub exp: f64,
}

impl Default for PetStats {
    fn default() -> Self {
        let level = 1;
        let feeling_max = Self::feeling_max_for_level(level);
        let strength_max = Self::strength_max_for_level(level);

        Self {
            health: 100.0,
            feeling: 100.0,
            feeling_max,
            strength: strength_max,
            strength_max,
            strength_food: 100.0,
            strength_drink: 100.0,
            likability: 0.0,
            level,
            exp: 0.0,
        }
    }
}

impl PetStats {
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

    pub fn level_up_exp_needed(&self) -> f64 {
        200.0 * self.level as f64 - 100.0
    }

    pub fn feeling_max_for_level(level: u32) -> f64 {
        let level_f = level as f64;
        100.0 + (level_f * (1.0 + level_f)).powf(0.75) * 2.0
    }

    pub fn strength_max_for_level(level: u32) -> f64 {
        let level_f = level as f64;
        100.0 + (level_f * (1.0 + level_f)).powf(0.75) * 4.0
    }
}
