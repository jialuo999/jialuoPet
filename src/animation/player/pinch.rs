use std::path::{Path, PathBuf};

use crate::stats_panel::PetMode;

use super::{AnimationPlayer, StartupPlayer, TouchPlayer};
use crate::animation::assets::{
    collect_pinch_end_files, collect_pinch_loop_variants, collect_pinch_start_files,
    pseudo_random_index,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum PinchPlaybackMode {
    None,
    Start,
    Loop,
    End,
}

pub(crate) struct PinchPlayer {
    pinch_root: PathBuf,
    pinch_start_files: Vec<PathBuf>,
    pinch_start_index: usize,
    pinch_loop_variants: Vec<Vec<PathBuf>>,
    pinch_loop_files: Vec<PathBuf>,
    pinch_loop_index: usize,
    pinch_end_files: Vec<PathBuf>,
    pinch_end_index: usize,
    playback_mode: PinchPlaybackMode,
}

impl PinchPlayer {
    pub(crate) fn new(pinch_root: PathBuf, mode: PetMode) -> Self {
        let pinch_start_files = collect_pinch_start_files(Path::new(&pinch_root), mode);
        let pinch_loop_variants = collect_pinch_loop_variants(Path::new(&pinch_root), mode);
        let pinch_end_files = collect_pinch_end_files(Path::new(&pinch_root), mode);

        Self {
            pinch_root,
            pinch_start_files,
            pinch_start_index: 0,
            pinch_loop_variants,
            pinch_loop_files: Vec::new(),
            pinch_loop_index: 0,
            pinch_end_files,
            pinch_end_index: 0,
            playback_mode: PinchPlaybackMode::None,
        }
    }

    pub(crate) fn start(&mut self, touch: &mut TouchPlayer, startup: &mut StartupPlayer) {
        if !self.pinch_start_files.is_empty() {
            startup.stop();
            touch.stop();
            self.playback_mode = PinchPlaybackMode::Start;
            self.pinch_start_index = 0;
            return;
        }

        if !self.pinch_loop_variants.is_empty() {
            startup.stop();
            touch.stop();
            let variant_index = pseudo_random_index(self.pinch_loop_variants.len());
            self.pinch_loop_files = self.pinch_loop_variants[variant_index].clone();
            self.pinch_loop_index = 0;
            self.playback_mode = PinchPlaybackMode::Loop;
        }
    }

    pub(crate) fn continue_loop(&mut self, touch: &mut TouchPlayer, startup: &mut StartupPlayer) {
        if self.playback_mode == PinchPlaybackMode::Start || self.pinch_loop_variants.is_empty() {
            return;
        }

        startup.stop();
        touch.stop();

        if self.playback_mode != PinchPlaybackMode::Loop || self.pinch_loop_files.is_empty() {
            let variant_index = pseudo_random_index(self.pinch_loop_variants.len());
            self.pinch_loop_files = self.pinch_loop_variants[variant_index].clone();
            self.pinch_loop_index = 0;
        }
        self.playback_mode = PinchPlaybackMode::Loop;
    }

    pub(crate) fn end(&mut self, touch: &mut TouchPlayer, startup: &mut StartupPlayer) {
        if self.pinch_end_files.is_empty() {
            self.playback_mode = PinchPlaybackMode::None;
            return;
        }

        startup.stop();
        touch.stop();
        self.playback_mode = PinchPlaybackMode::End;
        self.pinch_end_index = 0;
    }
}

impl AnimationPlayer for PinchPlayer {
    fn is_active(&self) -> bool {
        self.playback_mode != PinchPlaybackMode::None
    }

    fn next_frame(&mut self) -> Option<PathBuf> {
        match self.playback_mode {
            PinchPlaybackMode::None => None,
            PinchPlaybackMode::Start => {
                if self.pinch_start_files.is_empty() {
                    self.playback_mode = PinchPlaybackMode::None;
                    return None;
                }

                let frame = self.pinch_start_files[self.pinch_start_index].clone();
                let next = self.pinch_start_index + 1;
                if next < self.pinch_start_files.len() {
                    self.pinch_start_index = next;
                } else if !self.pinch_loop_variants.is_empty() {
                    let variant_index = pseudo_random_index(self.pinch_loop_variants.len());
                    self.pinch_loop_files = self.pinch_loop_variants[variant_index].clone();
                    self.pinch_loop_index = 0;
                    self.playback_mode = PinchPlaybackMode::Loop;
                } else {
                    self.playback_mode = PinchPlaybackMode::None;
                }
                Some(frame)
            }
            PinchPlaybackMode::Loop => {
                if self.pinch_loop_files.is_empty() {
                    self.playback_mode = PinchPlaybackMode::None;
                    return None;
                }

                let frame = self.pinch_loop_files[self.pinch_loop_index].clone();
                let next = self.pinch_loop_index + 1;
                if next < self.pinch_loop_files.len() {
                    self.pinch_loop_index = next;
                } else if !self.pinch_loop_variants.is_empty() {
                    let variant_index = pseudo_random_index(self.pinch_loop_variants.len());
                    self.pinch_loop_files = self.pinch_loop_variants[variant_index].clone();
                    self.pinch_loop_index = 0;
                }
                Some(frame)
            }
            PinchPlaybackMode::End => {
                if self.pinch_end_files.is_empty() {
                    self.playback_mode = PinchPlaybackMode::None;
                    return None;
                }

                let frame = self.pinch_end_files[self.pinch_end_index].clone();
                let next = self.pinch_end_index + 1;
                if next < self.pinch_end_files.len() {
                    self.pinch_end_index = next;
                } else {
                    self.playback_mode = PinchPlaybackMode::None;
                }
                Some(frame)
            }
        }
    }

    fn stop(&mut self) {
        self.playback_mode = PinchPlaybackMode::None;
        self.pinch_loop_files.clear();
        self.pinch_start_index = 0;
        self.pinch_loop_index = 0;
        self.pinch_end_index = 0;
    }

    fn reload(&mut self, mode: PetMode) {
        self.pinch_start_files = collect_pinch_start_files(Path::new(&self.pinch_root), mode);
        self.pinch_start_index = 0;
        self.pinch_loop_variants = collect_pinch_loop_variants(Path::new(&self.pinch_root), mode);
        self.pinch_loop_files.clear();
        self.pinch_loop_index = 0;
        self.pinch_end_files = collect_pinch_end_files(Path::new(&self.pinch_root), mode);
        self.pinch_end_index = 0;
    }
}
