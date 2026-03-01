use std::fs;
use std::path::{Path, PathBuf};

use crate::stats::PetMode;

use super::AnimationPlayer;
use crate::animation::assets::{load_frames_with_fallback, pseudo_random_index, Segment};

#[derive(Clone, Copy, PartialEq, Eq)]
enum StatePhase {
    None,
    A { index: usize },
    B { index: usize, remaining: u8 },
    C { index: usize },
}

pub(crate) struct StatePlayer {
    state_root: PathBuf,
    current_mode: PetMode,
    state_variants: Vec<PathBuf>,
    phase: StatePhase,
    cooldown_ticks: u32,
    a_files: Vec<PathBuf>,
    b_files: Vec<PathBuf>,
    c_files: Vec<PathBuf>,
}

impl StatePlayer {
    pub(crate) fn new(state_root: PathBuf, mode: PetMode) -> Self {
        let state_variants = collect_state_variants(&state_root);
        Self {
            state_root,
            current_mode: mode,
            state_variants,
            phase: StatePhase::None,
            cooldown_ticks: Self::choose_cooldown_ticks(),
            a_files: Vec::new(),
            b_files: Vec::new(),
            c_files: Vec::new(),
        }
    }

    fn choose_loop_count(&self) -> u8 {
        2 + pseudo_random_index(2) as u8
    }

    fn choose_cooldown_ticks() -> u32 {
        180 + pseudo_random_index(181) as u32
    }

    fn finish_cycle(&mut self) {
        self.phase = StatePhase::None;
        self.cooldown_ticks = Self::choose_cooldown_ticks();
        self.a_files.clear();
        self.b_files.clear();
        self.c_files.clear();
    }

    fn try_start_cycle(&mut self) -> bool {
        if self.state_variants.is_empty() {
            return false;
        }

        let total = self.state_variants.len();
        let offset = pseudo_random_index(total);
        for step in 0..total {
            let index = (offset + step) % total;
            let root = &self.state_variants[index];

            let a_files = load_frames_with_fallback(root, self.current_mode, Segment::A);
            let b_files = load_frames_with_fallback(root, self.current_mode, Segment::B);
            let c_files = load_frames_with_fallback(root, self.current_mode, Segment::C);

            if b_files.is_empty() {
                continue;
            }

            self.a_files = a_files;
            self.b_files = b_files;
            self.c_files = c_files;
            self.phase = if self.a_files.is_empty() {
                StatePhase::B {
                    index: 0,
                    remaining: self.choose_loop_count(),
                }
            } else {
                StatePhase::A { index: 0 }
            };
            return true;
        }

        false
    }
}

impl AnimationPlayer for StatePlayer {
    fn is_active(&self) -> bool {
        !self.state_variants.is_empty()
    }

    fn next_frame(&mut self) -> Option<PathBuf> {
        if self.phase == StatePhase::None {
            if self.cooldown_ticks > 0 {
                self.cooldown_ticks -= 1;
                return None;
            }
            if !self.try_start_cycle() {
                self.cooldown_ticks = Self::choose_cooldown_ticks();
                return None;
            }
        }

        match self.phase {
            StatePhase::None => None,
            StatePhase::A { mut index } => {
                if self.a_files.is_empty() {
                    self.phase = StatePhase::B {
                        index: 0,
                        remaining: self.choose_loop_count(),
                    };
                    return self.b_files.first().cloned();
                }

                let frame = self.a_files.get(index).cloned();
                let next = index + 1;
                if next < self.a_files.len() {
                    index = next;
                    self.phase = StatePhase::A { index };
                } else {
                    self.phase = StatePhase::B {
                        index: 0,
                        remaining: self.choose_loop_count(),
                    };
                }
                frame
            }
            StatePhase::B {
                mut index,
                mut remaining,
            } => {
                if self.b_files.is_empty() {
                    if self.c_files.is_empty() {
                        self.finish_cycle();
                        return None;
                    }
                    self.phase = StatePhase::C { index: 0 };
                    return self.c_files.first().cloned();
                }

                let frame = self.b_files.get(index).cloned();
                let next = index + 1;
                if next < self.b_files.len() {
                    index = next;
                    self.phase = StatePhase::B { index, remaining };
                } else if remaining > 1 {
                    remaining -= 1;
                    self.phase = StatePhase::B {
                        index: 0,
                        remaining,
                    };
                } else if self.c_files.is_empty() {
                    self.finish_cycle();
                } else {
                    self.phase = StatePhase::C { index: 0 };
                }
                frame
            }
            StatePhase::C { mut index } => {
                if self.c_files.is_empty() {
                    self.finish_cycle();
                    return None;
                }

                let frame = self.c_files.get(index).cloned();
                let next = index + 1;
                if next < self.c_files.len() {
                    index = next;
                    self.phase = StatePhase::C { index };
                } else {
                    self.finish_cycle();
                }
                frame
            }
        }
    }

    fn interrupt(&mut self, skip_to_end: bool) {
        if !skip_to_end && !self.c_files.is_empty() {
            self.phase = StatePhase::C { index: 0 };
            self.cooldown_ticks = 0;
            return;
        }

        self.phase = StatePhase::None;
        self.cooldown_ticks = Self::choose_cooldown_ticks();
        self.a_files.clear();
        self.b_files.clear();
        self.c_files.clear();
    }

    fn reload(&mut self, mode: PetMode) {
        self.current_mode = mode;
        self.state_variants = collect_state_variants(&self.state_root);
        self.phase = StatePhase::None;
        self.cooldown_ticks = Self::choose_cooldown_ticks();
        self.a_files.clear();
        self.b_files.clear();
        self.c_files.clear();
    }
}

fn collect_state_variants(state_root: &Path) -> Vec<PathBuf> {
    let mut variants: Vec<PathBuf> = fs::read_dir(state_root)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(|entry| entry.ok().map(|item| item.path())))
        .filter(|path| path.is_dir())
        .collect();
    variants.sort();
    variants
}