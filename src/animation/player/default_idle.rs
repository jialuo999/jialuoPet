impl DefaultIdlePlayer {
    // 根据显示类型选择帧间隔：状态动画更紧凑，其它走全局轮播间隔。
    pub fn frame_interval(&self) -> u64 {
        match self.display_type {
            DisplayGraphType::StateOne | DisplayGraphType::StateTwo => 250,
            _ => crate::config::CAROUSEL_INTERVAL_MS,
        }
    }
}
use std::path::PathBuf;

use rand::Rng;

use crate::animation::assets::{
    collect_idel_action_names, load_idel_loop_variants, load_idel_segment,
    load_state_loop_variants, load_state_segment, load_switch_single, pseudo_random_index,
    IdelStateSegment,
};
use crate::config::AnimationPathConfig;
use crate::stats::PetMode;

use super::AnimationPlayer;
use crate::animation::assets::{
    collect_default_happy_idle_variants, collect_default_mode_idle_variants,
    select_default_files_for_mode,
};

#[derive(Clone, Copy, PartialEq, Eq)]
// 当前正在显示的动画图谱类型，用于限制触发条件与帧间隔策略。
enum DisplayGraphType {
    Default,
    Idel,
    StateOne,
    StateTwo,
    SwitchUp,
    SwitchDown,
}

#[derive(Clone, Copy, PartialEq, Eq)]
// 模式切换动画方向：状态变差（Down）或恢复（Up）。
enum SwitchDirection {
    Up,
    Down,
}

#[derive(Clone)]
// 默认待机播放器的内部播放状态机。
// 注意：此状态描述的是“序列阶段”，不是最终 PetMode。
enum PlaybackState {
    Default,
    IdelStart { name: String },
    IdelLoop {
        name: String,
        loop_times: u32,
        duration: u32,
    },
    IdelEnd,
    IdelSingle,
    StateOneStart { count_nomal: u32 },
    StateOneLoop {
        loop_times: u32,
        duration: u32,
        count_nomal: u32,
    },
    StateOneEnd,
    StateTwoStart { count_nomal: u32 },
    StateTwoLoop {
        loop_times: u32,
        duration: u32,
        count_nomal: u32,
    },
    StateTwoEnd { count_nomal: u32 },
    Switch {
        before: PetMode,
        after: PetMode,
        direction: SwitchDirection,
    },
}

pub(crate) struct DefaultIdlePlayer {
    // 运行时配置（资源根目录、子目录约定等）。
    config: AnimationPathConfig,
    // 当前生效的宠物模式（影响资源选择）。
    current_mode: PetMode,
    // 默认待机资源池（按模式分组）。
    default_happy_variants: Vec<Vec<PathBuf>>,
    default_nomal_variants: Vec<Vec<PathBuf>>,
    default_poor_condition_variants: Vec<Vec<PathBuf>>,
    default_ill_variants: Vec<Vec<PathBuf>>,
    // 当前模式下选中的默认帧序列。
    default_files: Vec<PathBuf>,
    // 当前显示类型与播放状态。
    display_type: DisplayGraphType,
    playback_state: PlaybackState,
    // 当前激活序列与索引。
    active_frames: Vec<PathBuf>,
    active_index: usize,
    // 各类动画资源根目录缓存，避免频繁拼接路径。
    idel_root: PathBuf,
    state_root: PathBuf,
    switch_up_root: PathBuf,
    switch_down_root: PathBuf,
    // 不能立即切换时，暂存一次待处理请求。
    pending_mode_switch: Option<(PetMode, PetMode)>,
}

impl DefaultIdlePlayer {
    pub(crate) fn new(config: &AnimationPathConfig, mode: PetMode) -> Result<Self, String> {
        let default_happy_variants = collect_default_happy_idle_variants(config)?;
        if default_happy_variants.is_empty() {
            return Err("默认静息动画目录中没有找到 PNG 文件".to_string());
        }

        let default_nomal_variants = collect_default_mode_idle_variants(config, PetMode::Nomal);
        let default_poor_condition_variants =
            collect_default_mode_idle_variants(config, PetMode::PoorCondition);
        let default_ill_variants = collect_default_mode_idle_variants(config, PetMode::Ill);

        let default_files = select_default_files_for_mode(
            mode,
            &default_happy_variants,
            &default_nomal_variants,
            &default_poor_condition_variants,
            &default_ill_variants,
        );

        let mut player = Self {
            config: config.clone(),
            current_mode: mode,
            default_happy_variants,
            default_nomal_variants,
            default_poor_condition_variants,
            default_ill_variants,
            default_files,
            display_type: DisplayGraphType::Default,
            playback_state: PlaybackState::Default,
            active_frames: Vec::new(),
            active_index: 0,
            idel_root: PathBuf::from(&config.assets_body_root).join(&config.idel_root),
            state_root: PathBuf::from(&config.assets_body_root).join(&config.state_root),
            switch_up_root: PathBuf::from(&config.assets_body_root).join(&config.switch_up_root),
            switch_down_root: PathBuf::from(&config.assets_body_root).join(&config.switch_down_root),
            pending_mode_switch: None,
        };

        player.start_default();
        Ok(player)
    }

    fn refresh_selection(&mut self) {
        // 按 current_mode 重新挑选默认待机帧。
        self.default_files = select_default_files_for_mode(
            self.current_mode,
            &self.default_happy_variants,
            &self.default_nomal_variants,
            &self.default_poor_condition_variants,
            &self.default_ill_variants,
        );
    }

    fn set_frames(&mut self, frames: Vec<PathBuf>) {
        // 切换序列时总是从首帧开始。
        self.active_frames = frames;
        self.active_index = 0;
    }

    fn mode_rank(mode: PetMode) -> i32 {
        match mode {
            PetMode::Happy => 0,
            PetMode::Nomal => 1,
            PetMode::PoorCondition => 2,
            PetMode::Ill => 3,
        }
    }

    fn mode_from_rank(rank: i32) -> PetMode {
        match rank {
            i if i <= 0 => PetMode::Happy,
            1 => PetMode::Nomal,
            2 => PetMode::PoorCondition,
            _ => PetMode::Ill,
        }
    }

    fn can_switch_now(&self) -> bool {
        // 仅允许在默认态或切换链路中继续切换，避免打断动作序列。
        matches!(
            self.display_type,
            DisplayGraphType::Default | DisplayGraphType::SwitchUp | DisplayGraphType::SwitchDown
        )
    }

    fn start_default(&mut self) {
        // 回到稳定默认态，并尝试消费挂起的模式切换。
        self.display_type = DisplayGraphType::Default;
        self.playback_state = PlaybackState::Default;
        self.refresh_selection();
        self.set_frames(self.default_files.clone());
        self.try_consume_pending_mode_switch();
    }

    fn try_consume_pending_mode_switch(&mut self) {
        let Some((before, after)) = self.pending_mode_switch.take() else {
            return;
        };

        if self.can_switch_now() {
            self.start_switch_step(before, after);
        } else {
            self.pending_mode_switch = Some((before, after));
        }
    }

    fn should_end_loop(next_loop_times: u32, duration: u32) -> bool {
        // loop 次数越高越容易结束，duration 作为“耐久度”上限。
        if next_loop_times == 0 {
            return false;
        }

        let mut rng = rand::thread_rng();
        let sample = rng.gen_range(0..next_loop_times);
        sample > duration
    }

    fn choose_idel_action_name(&self) -> Option<String> {
        // 从 IDEL 动作目录中伪随机选择一个动作名。
        let names = collect_idel_action_names(&self.idel_root);
        if names.is_empty() {
            return None;
        }

        Some(names[pseudo_random_index(names.len())].clone())
    }

    fn start_idel_by_name(&mut self, name: String) -> bool {
        // 优先走 A(起手) -> B(循环) -> C(收尾) 的完整链路。
        let start_frames = load_idel_segment(
            &self.idel_root,
            &name,
            self.current_mode,
            IdelStateSegment::A,
        );
        if !start_frames.is_empty() {
            self.display_type = DisplayGraphType::Idel;
            self.playback_state = PlaybackState::IdelStart { name };
            self.set_frames(start_frames);
            return true;
        }

        // 若没有起手段，则退化为 Single 单段动作。
        let single_frames = load_idel_segment(
            &self.idel_root,
            &name,
            self.current_mode,
            IdelStateSegment::Single,
        );
        if single_frames.is_empty() {
            return false;
        }

        self.display_type = DisplayGraphType::Idel;
        self.playback_state = PlaybackState::IdelSingle;
        self.set_frames(single_frames);
        true
    }

    fn start_idel_loop(&mut self, name: String, loop_times: u32) {
        let loop_variants = load_idel_loop_variants(&self.idel_root, &name, self.current_mode);
        if loop_variants.is_empty() {
            self.start_default();
            return;
        }

        // duration 越大，结束概率越低，形成不同停留时长。
        let duration = (loop_variants.len().max(1) as u32) * 2;
        let loop_frames = loop_variants[pseudo_random_index(loop_variants.len())].clone();
        if loop_frames.is_empty() {
            self.start_default();
            return;
        }

        self.display_type = DisplayGraphType::Idel;
        self.playback_state = PlaybackState::IdelLoop {
            name,
            loop_times,
            duration,
        };
        self.set_frames(loop_frames);
    }

    fn start_idel_end(&mut self, name: &str) {
        let end_frames = load_idel_segment(
            &self.idel_root,
            name,
            self.current_mode,
            IdelStateSegment::C,
        );
        if end_frames.is_empty() {
            self.start_default();
            return;
        }

        self.display_type = DisplayGraphType::Idel;
        self.playback_state = PlaybackState::IdelEnd;
        self.set_frames(end_frames);
    }

    fn start_state_one_start(&mut self, count_nomal: u32) {
        let start_frames = load_state_segment(
            &self.state_root,
            "StateONE",
            self.current_mode,
            IdelStateSegment::A,
        );
        if start_frames.is_empty() {
            self.start_default();
            return;
        }

        self.display_type = DisplayGraphType::StateOne;
        self.playback_state = PlaybackState::StateOneStart { count_nomal };
        self.set_frames(start_frames);
    }

    fn start_state_one_loop(&mut self, loop_times: u32, count_nomal: u32) {
        let loop_variants = load_state_loop_variants(&self.state_root, "StateONE", self.current_mode);
        if loop_variants.is_empty() {
            self.start_default();
            return;
        }

        let duration = loop_variants.len().max(1) as u32;
        let loop_frames = loop_variants[pseudo_random_index(loop_variants.len())].clone();
        if loop_frames.is_empty() {
            self.start_default();
            return;
        }

        self.display_type = DisplayGraphType::StateOne;
        self.playback_state = PlaybackState::StateOneLoop {
            loop_times,
            duration,
            count_nomal,
        };
        self.set_frames(loop_frames);
    }

    fn start_state_one_end(&mut self) {
        let end_frames = load_state_segment(
            &self.state_root,
            "StateONE",
            self.current_mode,
            IdelStateSegment::C,
        );
        if end_frames.is_empty() {
            self.start_default();
            return;
        }

        self.display_type = DisplayGraphType::StateOne;
        self.playback_state = PlaybackState::StateOneEnd;
        self.set_frames(end_frames);
    }

    fn start_state_two_start(&mut self, count_nomal: u32) {
        let start_frames = load_state_segment(
            &self.state_root,
            "StateTWO",
            self.current_mode,
            IdelStateSegment::A,
        );
        if start_frames.is_empty() {
            self.start_state_one_loop(1, count_nomal);
            return;
        }

        self.display_type = DisplayGraphType::StateTwo;
        self.playback_state = PlaybackState::StateTwoStart { count_nomal };
        self.set_frames(start_frames);
    }

    fn start_state_two_loop(&mut self, loop_times: u32, count_nomal: u32) {
        let loop_variants = load_state_loop_variants(&self.state_root, "StateTWO", self.current_mode);
        if loop_variants.is_empty() {
            self.start_state_one_loop(1, count_nomal);
            return;
        }

        let duration = loop_variants.len().max(1) as u32;
        let loop_frames = loop_variants[pseudo_random_index(loop_variants.len())].clone();
        if loop_frames.is_empty() {
            self.start_state_one_loop(1, count_nomal);
            return;
        }

        self.display_type = DisplayGraphType::StateTwo;
        self.playback_state = PlaybackState::StateTwoLoop {
            loop_times,
            duration,
            count_nomal,
        };
        self.set_frames(loop_frames);
    }

    fn start_state_two_end(&mut self, count_nomal: u32) {
        let end_frames = load_state_segment(
            &self.state_root,
            "StateTWO",
            self.current_mode,
            IdelStateSegment::C,
        );
        if end_frames.is_empty() {
            self.start_state_one_loop(1, count_nomal);
            return;
        }

        self.display_type = DisplayGraphType::StateTwo;
        self.playback_state = PlaybackState::StateTwoEnd { count_nomal };
        self.set_frames(end_frames);
    }

    fn start_switch_step(&mut self, before: PetMode, after: PetMode) {
        // 采用逐级切换：例如 Happy -> Ill 会依次经过中间模式。
        if before == after {
            self.start_default();
            return;
        }

        let before_rank = Self::mode_rank(before);
        let after_rank = Self::mode_rank(after);
        if before_rank < after_rank {
            let frames = load_switch_single(&self.switch_down_root, before);
            if frames.is_empty() {
                self.start_switch_step(Self::mode_from_rank(before_rank + 1), after);
                return;
            }

            self.display_type = DisplayGraphType::SwitchDown;
            self.playback_state = PlaybackState::Switch {
                before,
                after,
                direction: SwitchDirection::Down,
            };
            self.set_frames(frames);
            return;
        }

        let frames = load_switch_single(&self.switch_up_root, before);
        if frames.is_empty() {
            self.start_switch_step(Self::mode_from_rank(before_rank - 1), after);
            return;
        }

        self.display_type = DisplayGraphType::SwitchUp;
        self.playback_state = PlaybackState::Switch {
            before,
            after,
            direction: SwitchDirection::Up,
        };
        self.set_frames(frames);
    }

    fn on_active_sequence_finished(&mut self) {
        // 使用快照避免 match 分支中修改 self 时发生借用冲突。
        let snapshot = self.playback_state.clone();

        match snapshot {
            PlaybackState::Default => {
                self.active_index = 0;
            }
            PlaybackState::IdelStart { name } => {
                self.start_idel_loop(name, 1);
            }
            PlaybackState::IdelLoop {
                name,
                loop_times,
                duration,
            } => {
                let next_loop_times = loop_times.saturating_add(1);
                if Self::should_end_loop(next_loop_times, duration) {
                    self.start_idel_end(&name);
                } else {
                    self.start_idel_loop(name, next_loop_times);
                }
            }
            PlaybackState::IdelEnd | PlaybackState::IdelSingle => {
                self.start_default();
            }
            PlaybackState::StateOneStart { count_nomal } => {
                self.start_state_one_loop(1, count_nomal);
            }
            PlaybackState::StateOneLoop {
                loop_times,
                duration,
                count_nomal,
            } => {
                let next_loop_times = loop_times.saturating_add(1);
                if Self::should_end_loop(next_loop_times, duration) {
                    // StateONE 结束后，按概率进入 StateTWO 或直接收尾。
                    // count_nomal 越大，进入 StateTWO 的概率越低。
                    let upper = 1_u32.saturating_add(count_nomal);
                    let branch = rand::thread_rng().gen_range(0..=upper);
                    if branch == 0 {
                        self.start_state_two_start(count_nomal);
                    } else {
                        self.start_state_one_end();
                    }
                } else {
                    self.start_state_one_loop(next_loop_times, count_nomal);
                }
            }
            PlaybackState::StateOneEnd => {
                self.start_default();
            }
            PlaybackState::StateTwoStart { count_nomal } => {
                self.start_state_two_loop(1, count_nomal);
            }
            PlaybackState::StateTwoLoop {
                loop_times,
                duration,
                count_nomal,
            } => {
                let next_loop_times = loop_times.saturating_add(1);
                if Self::should_end_loop(next_loop_times, duration) {
                    self.start_state_two_end(count_nomal);
                } else {
                    self.start_state_two_loop(next_loop_times, count_nomal);
                }
            }
            PlaybackState::StateTwoEnd { count_nomal } => {
                self.start_state_one_loop(1, count_nomal);
            }
            PlaybackState::Switch {
                before,
                after,
                direction,
            } => {
                // 单步切换结束后推进到下一个中间模式，直到抵达 after。
                let before_rank = Self::mode_rank(before);
                let next_before = match direction {
                    SwitchDirection::Down => Self::mode_from_rank(before_rank + 1),
                    SwitchDirection::Up => Self::mode_from_rank(before_rank - 1),
                };
                self.start_switch_step(next_before, after);
            }
        }
    }

    pub(crate) fn enter(&mut self) -> Option<PathBuf> {
        self.start_default();
        self.active_frames.first().cloned()
    }

    pub(crate) fn trigger_idel(&mut self) -> bool {
        // 仅允许从默认态触发，避免打断状态机关键序列。
        if self.display_type != DisplayGraphType::Default {
            return false;
        }

        let Some(name) = self.choose_idel_action_name() else {
            return false;
        };

        self.start_idel_by_name(name)
    }

    pub(crate) fn trigger_state_one(&mut self, count_nomal: u32) -> bool {
        if self.display_type != DisplayGraphType::Default {
            return false;
        }

        self.start_state_one_start(count_nomal);
        self.display_type == DisplayGraphType::StateOne
    }

    pub(crate) fn request_mode_switch(&mut self, before: PetMode, after: PetMode) {
        // 先更新目标模式，确保后续资源选择与最终目标一致。
        self.current_mode = after;

        if before == after {
            self.start_default();
            return;
        }

        if !self.can_switch_now() {
            self.pending_mode_switch = Some((before, after));
            return;
        }

        self.pending_mode_switch = None;
        self.start_switch_step(before, after);
    }
}

impl AnimationPlayer for DefaultIdlePlayer {
    fn is_active(&self) -> bool {
        true
    }

    fn next_frame(&mut self) -> Option<PathBuf> {
        if self.active_frames.is_empty() {
            self.start_default();
        }
        if self.active_frames.is_empty() {
            return None;
        }

        let frame = self.active_frames.get(self.active_index).cloned();
        if frame.is_none() {
            return None;
        }

        let next = self.active_index + 1;
        if next < self.active_frames.len() {
            self.active_index = next;
        } else {
            self.on_active_sequence_finished();
        }

        frame
    }

    fn interrupt(&mut self, _skip_to_end: bool) {
        self.start_default();
    }

    fn reload(&mut self, mode: PetMode) {
        self.current_mode = mode;
        self.default_happy_variants =
            collect_default_happy_idle_variants(&self.config).unwrap_or_default();
        self.default_nomal_variants = collect_default_mode_idle_variants(&self.config, PetMode::Nomal);
        self.default_poor_condition_variants =
            collect_default_mode_idle_variants(&self.config, PetMode::PoorCondition);
        self.default_ill_variants = collect_default_mode_idle_variants(&self.config, PetMode::Ill);
        self.idel_root = PathBuf::from(&self.config.assets_body_root).join(&self.config.idel_root);
        self.state_root = PathBuf::from(&self.config.assets_body_root).join(&self.config.state_root);
        self.switch_up_root = PathBuf::from(&self.config.assets_body_root).join(&self.config.switch_up_root);
        self.switch_down_root =
            PathBuf::from(&self.config.assets_body_root).join(&self.config.switch_down_root);
        self.start_default();
    }
}
