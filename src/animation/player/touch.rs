use std::path::{Path, PathBuf};

use crate::stats::PetMode;

use super::{AnimationPlayer, StartupPlayer};
use crate::animation::assets::{build_touch_sequence, collect_touch_variants, TouchStageVariants};

#[derive(Clone, Copy, PartialEq, Eq)]
enum TouchPlaybackMode {
    None,
    Head,
    Body,
}

pub(crate) struct TouchPlayer {
    touch_head_root: PathBuf,
    touch_body_root: PathBuf,
    touch_head_variants: TouchStageVariants,
    touch_body_variants: TouchStageVariants,
    touch_files: Vec<PathBuf>,
    touch_index: usize,
    playback_mode: TouchPlaybackMode,
}

impl TouchPlayer {
    pub(crate) fn new(touch_head_root: PathBuf, touch_body_root: PathBuf, mode: PetMode) -> Self {
        let touch_head_variants = collect_touch_variants(Path::new(&touch_head_root), mode);
        let touch_body_variants = collect_touch_variants(Path::new(&touch_body_root), mode);

        Self {
            touch_head_root,
            touch_body_root,
            touch_head_variants,
            touch_body_variants,
            touch_files: Vec::new(),
            touch_index: 0,
            playback_mode: TouchPlaybackMode::None,
        }
    }

    pub(crate) fn start_head(&mut self, startup: &mut StartupPlayer) {
        let sequence = build_touch_sequence(&self.touch_head_variants);
        if sequence.is_empty() {
            return;
        }

        startup.stop();
        self.playback_mode = TouchPlaybackMode::Head;
        self.touch_files = sequence;
        self.touch_index = 0;
    }

    pub(crate) fn start_body(&mut self, startup: &mut StartupPlayer) {
        let sequence = build_touch_sequence(&self.touch_body_variants);
        if sequence.is_empty() {
            return;
        }

        startup.stop();
        self.playback_mode = TouchPlaybackMode::Body;
        self.touch_files = sequence;
        self.touch_index = 0;
    }
}

impl AnimationPlayer for TouchPlayer {
    fn is_active(&self) -> bool {
        self.playback_mode != TouchPlaybackMode::None
    }

    fn next_frame(&mut self) -> Option<PathBuf> {
        if !self.is_active() || self.touch_files.is_empty() {
            self.stop();
            return None;
        }

        let frame = self.touch_files[self.touch_index].clone();
        let next = self.touch_index + 1;
        if next < self.touch_files.len() {
            self.touch_index = next;
        } else {
            self.stop();
        }
        Some(frame)
    }

    fn interrupt(&mut self, _skip_to_end: bool) {
        self.playback_mode = TouchPlaybackMode::None;
        self.touch_files.clear();
        self.touch_index = 0;
    }

    fn reload(&mut self, mode: PetMode) {
        self.touch_head_variants = collect_touch_variants(Path::new(&self.touch_head_root), mode);
        self.touch_body_variants = collect_touch_variants(Path::new(&self.touch_body_root), mode);
        self.interrupt(true);
    }
}
