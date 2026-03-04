use std::path::{Path, PathBuf};

use rand::Rng;

use crate::animation::assets::{
    collect_side_hide_end_files, collect_side_hide_loop_variants, collect_side_hide_start_files,
};
use crate::stats::PetMode;

use super::AnimationPlayer;

#[derive(Clone, Copy, PartialEq, Eq)]
enum SideHidePlaybackMode {
    None,
    Start,
    Loop,
    End,
}

pub(crate) struct SideHideRightMainPlayer {
    side_hide_root: PathBuf,
    side_hide_start_files: Vec<PathBuf>,
    side_hide_start_index: usize,
    side_hide_loop_variants: Vec<Vec<PathBuf>>,
    next_loop_variant_index: usize,
    current_loop_variant_index: Option<usize>,
    replay_current_loop_variant_once: bool,
    side_hide_loop_files: Vec<PathBuf>,
    side_hide_loop_index: usize,
    side_hide_end_files: Vec<PathBuf>,
    side_hide_end_index: usize,
    playback_mode: SideHidePlaybackMode,
}

impl SideHideRightMainPlayer {
    pub(crate) fn new(side_hide_root: PathBuf, mode: PetMode) -> Self {
        let side_hide_start_files = collect_side_hide_start_files(Path::new(&side_hide_root), mode);
        let side_hide_loop_variants = collect_side_hide_loop_variants(Path::new(&side_hide_root), mode);
        let side_hide_end_files = collect_side_hide_end_files(Path::new(&side_hide_root), mode);

        Self {
            side_hide_root,
            side_hide_start_files,
            side_hide_start_index: 0,
            side_hide_loop_variants,
            next_loop_variant_index: 0,
            current_loop_variant_index: None,
            replay_current_loop_variant_once: false,
            side_hide_loop_files: Vec::new(),
            side_hide_loop_index: 0,
            side_hide_end_files,
            side_hide_end_index: 0,
            playback_mode: SideHidePlaybackMode::None,
        }
    }

    pub(crate) fn start(&mut self) {
        if self.playback_mode != SideHidePlaybackMode::None {
            return;
        }

        if !self.side_hide_start_files.is_empty() {
            self.playback_mode = SideHidePlaybackMode::Start;
            self.side_hide_start_index = 0;
            return;
        }

        self.start_loop_segment();
    }

    fn start_loop_segment(&mut self) {
        if self.side_hide_loop_variants.is_empty() {
            self.playback_mode = SideHidePlaybackMode::None;
            self.side_hide_loop_files.clear();
            self.side_hide_loop_index = 0;
            return;
        }

        let variants_len = self.side_hide_loop_variants.len();
        let variant_index = if self.replay_current_loop_variant_once {
            self.replay_current_loop_variant_once = false;
            self.current_loop_variant_index.unwrap_or(self.next_loop_variant_index % variants_len)
        } else {
            let index = self.next_loop_variant_index % variants_len;
            self.next_loop_variant_index = (self.next_loop_variant_index + 1) % variants_len;
            index
        };

        self.current_loop_variant_index = Some(variant_index);
        self.side_hide_loop_files = self.side_hide_loop_variants[variant_index].clone();
        self.side_hide_loop_index = 0;
        self.replay_current_loop_variant_once = Self::should_replay_left_main_happy_b4(
            &self.side_hide_root,
            &self.side_hide_loop_files,
        ) && rand::thread_rng().gen_bool(0.8); // 80% 概率重播一次当前变体，增加停留时长的随机性
        self.playback_mode = SideHidePlaybackMode::Loop;
    }

    fn should_replay_left_main_happy_b4(side_hide_root: &Path, files: &[PathBuf]) -> bool {
        let is_left_main = side_hide_root
            .components()
            .any(|component| {
                component
                    .as_os_str()
                    .to_str()
                    .map(|name| name.eq_ignore_ascii_case("SideHide_Left_Main"))
                    .unwrap_or(false)
            });

        if !is_left_main {
            return false;
        }

        files.iter().any(|path| {
            let mut has_happy = false;
            let mut has_b4 = false;

            for component in path.components() {
                let Some(name) = component.as_os_str().to_str() else {
                    continue;
                };
                if name.eq_ignore_ascii_case("Happy") {
                    has_happy = true;
                }
                if name.eq_ignore_ascii_case("B_4") {
                    has_b4 = true;
                }
            }

            has_happy && has_b4
        })
    }
}

impl AnimationPlayer for SideHideRightMainPlayer {
    fn is_active(&self) -> bool {
        self.playback_mode != SideHidePlaybackMode::None
    }

    fn next_frame(&mut self) -> Option<PathBuf> {
        match self.playback_mode {
            SideHidePlaybackMode::None => None,
            SideHidePlaybackMode::Start => {
                if self.side_hide_start_files.is_empty() {
                    self.start_loop_segment();
                    return self.next_frame();
                }

                let frame = self.side_hide_start_files[self.side_hide_start_index].clone();
                let next = self.side_hide_start_index + 1;
                if next < self.side_hide_start_files.len() {
                    self.side_hide_start_index = next;
                } else {
                    self.start_loop_segment();
                }
                Some(frame)
            }
            SideHidePlaybackMode::Loop => {
                if self.side_hide_loop_files.is_empty() {
                    self.playback_mode = SideHidePlaybackMode::None;
                    return None;
                }

                let frame = self.side_hide_loop_files[self.side_hide_loop_index].clone();
                let next = self.side_hide_loop_index + 1;
                if next < self.side_hide_loop_files.len() {
                    self.side_hide_loop_index = next;
                } else {
                    self.start_loop_segment();
                }

                Some(frame)
            }
            SideHidePlaybackMode::End => {
                if self.side_hide_end_files.is_empty() {
                    self.playback_mode = SideHidePlaybackMode::None;
                    return None;
                }

                let frame = self.side_hide_end_files[self.side_hide_end_index].clone();
                let next = self.side_hide_end_index + 1;
                if next < self.side_hide_end_files.len() {
                    self.side_hide_end_index = next;
                } else {
                    self.playback_mode = SideHidePlaybackMode::None;
                }
                Some(frame)
            }
        }
    }

    fn interrupt(&mut self, skip_to_end: bool) {
        if !skip_to_end
            && matches!(
                self.playback_mode,
                SideHidePlaybackMode::Start | SideHidePlaybackMode::Loop
            )
            && !self.side_hide_end_files.is_empty()
        {
            self.playback_mode = SideHidePlaybackMode::End;
            self.side_hide_end_index = 0;
            return;
        }

        self.playback_mode = SideHidePlaybackMode::None;
        self.side_hide_start_index = 0;
        self.current_loop_variant_index = None;
        self.replay_current_loop_variant_once = false;
        self.next_loop_variant_index = 0;
        self.side_hide_loop_files.clear();
        self.side_hide_loop_index = 0;
        self.side_hide_end_index = 0;
    }

    fn reload(&mut self, mode: PetMode) {
        self.side_hide_start_files = collect_side_hide_start_files(Path::new(&self.side_hide_root), mode);
        self.side_hide_start_index = 0;
        self.side_hide_loop_variants = collect_side_hide_loop_variants(Path::new(&self.side_hide_root), mode);
        self.current_loop_variant_index = None;
        self.replay_current_loop_variant_once = false;
        self.next_loop_variant_index = 0;
        self.side_hide_loop_files.clear();
        self.side_hide_loop_index = 0;
        self.side_hide_end_files = collect_side_hide_end_files(Path::new(&self.side_hide_root), mode);
        self.side_hide_end_index = 0;
        self.playback_mode = SideHidePlaybackMode::None;
    }
}