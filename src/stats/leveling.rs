/// 宠物等级与成长状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PetLevelState {
    /// 当前等级
    pub level: u32,
    /// 等级上限（阶段）
    pub level_max: u32,
    /// 当前经验值
    pub exp: f64,
    /// 好感度上限
    pub likability_max: f64,
    /// 体力上限
    pub strength_max: f64,
    /// 心情上限
    pub feeling_max: f64,
}

/// 增加经验后的升级结果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AddExpResult {
    /// 是否升级
    pub leveled: bool,
    /// 是否阶段提升
    pub stage_up: bool,
}

impl Default for PetLevelState {
    /// 默认初始状态：1级，经验0，各上限100
    fn default() -> Self {
        let mut state = Self {
            level: 1,
            level_max: 0,
            exp: 0.0,
            likability_max: 100.0,
            strength_max: 100.0,
            feeling_max: 100.0,
        };
        recalc_caps(&mut state);
        state
    }
}

/// 计算当前等级升级所需经验
pub fn level_up_need(level: u32) -> f64 {
    200.0 * level as f64 - 100.0
}

/// 根据等级和阶段重新计算体力/心情上限
pub fn recalc_caps(state: &mut PetLevelState) {
    let base = (state.level as f64 * (1.0 + state.level_max as f64)).powf(0.75);
    state.strength_max = 100.0 + (base * 4.0).floor();
    state.feeling_max = 100.0 + (base * 2.0).floor();
}

/// 根据等级和阶段计算好感度上限
pub fn likability_max_from_level_state(level: u32, level_max: u32) -> f64 {
    if level_max == 0 {
        return 100.0 + (level.saturating_sub(1) as f64) * 10.0;
    }

    // 阶段起点
    let stage = level_max as u64;
    let level_u64 = level as u64;
    let stage_start_level = 100_u64.saturating_mul(stage);
    let stage_progress = level_u64.saturating_sub(stage_start_level);

    // 总升级数（含阶段累计）
    let total_level_ups = 1000_u64
        .saturating_add((stage.saturating_sub(1)).saturating_mul(1001_u64))
        .saturating_add(stage_progress);

    100.0 + total_level_ups as f64 * 10.0
}

/// 增加经验并判断是否升级/阶段提升
pub fn add_exp(state: &mut PetLevelState, delta_exp: f64) -> AddExpResult {
    state.exp += delta_exp;

    let mut need = level_up_need(state.level.max(1));
    let mut leveled = false;
    let mut stage_up = false;

    while state.exp >= need {
        leveled = true;
        state.exp -= need;
        state.likability_max += 10.0;
        state.level = state.level.saturating_add(1);

        let stage_gate = 1000_u32.saturating_add(state.level_max.saturating_mul(100));
        if state.level > stage_gate {
            state.level_max = state.level_max.saturating_add(1);
            state.level = 100_u32.saturating_mul(state.level_max).max(1);
            stage_up = true;
        }

        need = level_up_need(state.level.max(1));
    }

    recalc_caps(state);

    AddExpResult { leveled, stage_up }
}

#[cfg(test)]
mod tests {
    use super::{
        add_exp, level_up_need, likability_max_from_level_state, recalc_caps, PetLevelState,
    };

    #[test]
    fn add_exp_supports_multi_level_up() {
        let mut state = PetLevelState::default();
        let result = add_exp(&mut state, 2_000.0);
        assert!(result.leveled);
        assert_eq!(state.level, 5);
        assert_eq!(state.likability_max, 140.0);
        assert!(state.exp < level_up_need(state.level));
    }

    #[test]
    fn stage_transition_triggers_at_boundary() {
        let mut state = PetLevelState {
            level: 1000,
            level_max: 0,
            exp: 0.0,
            likability_max: 100.0 + 999.0 * 10.0,
            strength_max: 0.0,
            feeling_max: 0.0,
        };
        recalc_caps(&mut state);

        let result = add_exp(&mut state, level_up_need(1000));
        assert!(result.leveled);
        assert!(result.stage_up);
        assert_eq!(state.level_max, 1);
        assert_eq!(state.level, 100);
        assert_eq!(state.likability_max, 100.0 + 1_000.0 * 10.0);
    }

    #[test]
    fn likability_max_increases_by_ten_per_level() {
        let mut state = PetLevelState::default();
        add_exp(&mut state, level_up_need(1));
        assert_eq!(state.level, 2);
        assert_eq!(state.likability_max, 110.0);
    }

    #[test]
    fn caps_follow_formula_with_floor() {
        let mut state = PetLevelState {
            level: 100,
            level_max: 1,
            exp: 0.0,
            likability_max: 100.0,
            strength_max: 0.0,
            feeling_max: 0.0,
        };
        recalc_caps(&mut state);

        let base = (100.0_f64 * 2.0).powf(0.75);
        assert_eq!(state.strength_max, 100.0 + (base * 4.0).floor());
        assert_eq!(state.feeling_max, 100.0 + (base * 2.0).floor());
    }

    #[test]
    fn exp_is_always_below_next_need_after_upgrade() {
        let mut state = PetLevelState::default();
        add_exp(&mut state, 1_000_000.0);
        assert!(state.exp < level_up_need(state.level));
    }

    #[test]
    fn reconstruct_likability_max_for_stage_state() {
        let expected = 100.0 + 1000.0 * 10.0;
        assert_eq!(likability_max_from_level_state(100, 1), expected);
    }
}
