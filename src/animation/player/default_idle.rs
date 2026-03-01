use std::path::PathBuf;

use crate::config::AnimationPathConfig;
use crate::stats::PetMode;

use super::AnimationPlayer;
use crate::animation::assets::{
    body_asset_path, collect_default_happy_idle_variants, collect_default_mode_idle_variants,
    choose_idle_abc_sequence, pseudo_random_index, select_default_files_for_mode,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum IdlePhase {
    Default,
    A { index: usize },
    B { index: usize, remaining: u8 },
    C { index: usize },
}

pub(crate) struct DefaultIdlePlayer {
    config: AnimationPathConfig,
    current_mode: PetMode,
    idle_root: PathBuf,
    default_happy_variants: Vec<Vec<PathBuf>>,
    default_nomal_variants: Vec<Vec<PathBuf>>,
    default_poor_condition_variants: Vec<Vec<PathBuf>>,
    default_ill_variants: Vec<Vec<PathBuf>>,
    default_files: Vec<PathBuf>,
    default_index: usize,
    phase: IdlePhase,
    tick: u32,
    a_files: Vec<PathBuf>,
    b_files: Vec<PathBuf>,
    c_files: Vec<PathBuf>,
    idle_abc_just_finished: bool,
    idle_abc_cooldown_ticks: u32,
    idle_abc_cooldown_min_ticks: u32,
    idle_abc_cooldown_max_ticks: u32,
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

        let idle_root = body_asset_path(&config.assets_body_root, "IDEL");
        let idle_abc_cooldown_min_ticks = config.idel_abc_cooldown_min_ticks;
        let idle_abc_cooldown_max_ticks = config
            .idel_abc_cooldown_max_ticks
            .max(idle_abc_cooldown_min_ticks);
        let idle_abc_cooldown_ticks = {
            let span = (idle_abc_cooldown_max_ticks - idle_abc_cooldown_min_ticks) as usize + 1;
            idle_abc_cooldown_min_ticks + pseudo_random_index(span) as u32
        };
        Ok(Self {
            config: config.clone(),
            current_mode: mode,
            idle_root,
            default_happy_variants,
            default_nomal_variants,
            default_poor_condition_variants,
            default_ill_variants,
            default_files,
            default_index: 0,
            phase: IdlePhase::Default,
            tick: 0,
            a_files: Vec::new(),
            b_files: Vec::new(),
            c_files: Vec::new(),
            idle_abc_just_finished: false,
            idle_abc_cooldown_ticks,
            idle_abc_cooldown_min_ticks,
            idle_abc_cooldown_max_ticks,
        })
    }

    fn choose_idle_abc_cooldown(&self) -> u32 {
        let min_ticks = self.idle_abc_cooldown_min_ticks;
        let max_ticks = self.idle_abc_cooldown_max_ticks.max(min_ticks);
        let span = (max_ticks - min_ticks) as usize + 1;
        min_ticks + pseudo_random_index(span) as u32
    }

    fn refresh_selection(&mut self) {
        self.default_files = select_default_files_for_mode(
            self.current_mode,
            &self.default_happy_variants,
            &self.default_nomal_variants,
            &self.default_poor_condition_variants,
            &self.default_ill_variants,
        );
        self.default_index = 0;
    }

    pub(crate) fn enter(&mut self) -> Option<PathBuf> {
        self.refresh_selection();
        self.phase = IdlePhase::Default;
        self.tick = 0;
        self.default_index = 0;
        self.default_files.first().cloned()
    }

    fn next_default_frame(&mut self) -> Option<PathBuf> {
        if self.default_files.is_empty() {
            return None;
        }

        let next_index = (self.default_index + 1) % self.default_files.len();
        self.default_index = next_index;
        self.default_files.get(next_index).cloned()
    }

    pub(crate) fn take_idle_abc_finished(&mut self) -> bool {
        let finished = self.idle_abc_just_finished;
        self.idle_abc_just_finished = false;
        finished
    }

    pub(crate) fn is_playing_idle_abc(&self) -> bool {
        !matches!(self.phase, IdlePhase::Default)
    }
}

impl AnimationPlayer for DefaultIdlePlayer {
    fn is_active(&self) -> bool {
        true
    }

    fn next_frame(&mut self) -> Option<PathBuf> {
        self.tick = self.tick.wrapping_add(1);
        self.idle_abc_just_finished = false;

        match self.phase {
            IdlePhase::Default => {
                if self.idle_abc_cooldown_ticks > 0 {
                    self.idle_abc_cooldown_ticks -= 1;
                    return self.next_default_frame();
                }

                if let Some((a_files, b_files, c_files)) =
                    choose_idle_abc_sequence(&self.idle_root, self.current_mode)
                {
                    self.a_files = a_files;
                    self.b_files = b_files;
                    self.c_files = c_files;
                    self.phase = IdlePhase::A { index: 0 };
                    return self.a_files.first().cloned().or_else(|| self.next_default_frame());
                }
                self.next_default_frame()
            }
            IdlePhase::A { mut index } => {
                if self.a_files.is_empty() {
                    self.phase = IdlePhase::Default;
                    return self.next_default_frame();
                }

                let next = index + 1;
                if next < self.a_files.len() {
                    index = next;
                    self.phase = IdlePhase::A { index };
                    self.a_files.get(next).cloned()
                } else {
                    let repeats = 2 + pseudo_random_index(3) as u8;
                    self.phase = IdlePhase::B {
                        index: 0,
                        remaining: repeats,
                    };
                    self.b_files
                        .first()
                        .cloned()
                        .or_else(|| self.c_files.first().cloned())
                        .or_else(|| self.next_default_frame())
                }
            }
            IdlePhase::B {
                mut index,
                mut remaining,
            } => {
                if self.b_files.is_empty() {
                    self.phase = IdlePhase::C { index: 0 };
                    return self
                        .c_files
                        .first()
                        .cloned()
                        .or_else(|| self.next_default_frame());
                }

                let next = index + 1;
                if next < self.b_files.len() {
                    index = next;
                    self.phase = IdlePhase::B { index, remaining };
                    self.b_files.get(next).cloned()
                } else if remaining > 1 {
                    remaining -= 1;
                    self.phase = IdlePhase::B {
                        index: 0,
                        remaining,
                    };
                    self.b_files.first().cloned()
                } else {
                    self.phase = IdlePhase::C { index: 0 };
                    self.c_files
                        .first()
                        .cloned()
                        .or_else(|| self.next_default_frame())
                }
            }
            IdlePhase::C { mut index } => {
                if self.c_files.is_empty() {
                    self.phase = IdlePhase::Default;
                    self.default_index = 0;
                    self.idle_abc_just_finished = true;
                    self.idle_abc_cooldown_ticks = self.choose_idle_abc_cooldown();
                    return self.default_files.first().cloned();
                }

                let next = index + 1;
                if next < self.c_files.len() {
                    index = next;
                    self.phase = IdlePhase::C { index };
                    self.c_files.get(next).cloned()
                } else {
                    self.phase = IdlePhase::Default;
                    self.default_index = 0;
                    self.idle_abc_just_finished = true;
                    self.idle_abc_cooldown_ticks = self.choose_idle_abc_cooldown();
                    self.default_files.first().cloned()
                }
            }
        }
    }

    fn interrupt(&mut self, _skip_to_end: bool) {
        self.phase = IdlePhase::Default;
        self.tick = 0;
        self.idle_abc_just_finished = false;
        self.idle_abc_cooldown_ticks = self.choose_idle_abc_cooldown();
    }

    fn reload(&mut self, mode: PetMode) {
        self.current_mode = mode;
        self.default_happy_variants =
            collect_default_happy_idle_variants(&self.config).unwrap_or_default();
        self.default_nomal_variants = collect_default_mode_idle_variants(&self.config, PetMode::Nomal);
        self.default_poor_condition_variants =
            collect_default_mode_idle_variants(&self.config, PetMode::PoorCondition);
        self.default_ill_variants = collect_default_mode_idle_variants(&self.config, PetMode::Ill);
        self.a_files.clear();
        self.b_files.clear();
        self.c_files.clear();
        self.phase = IdlePhase::Default;
        self.tick = 0;
        self.idle_abc_just_finished = false;
        self.idle_abc_cooldown_ticks = self.choose_idle_abc_cooldown();
        self.refresh_selection();
    }
}
