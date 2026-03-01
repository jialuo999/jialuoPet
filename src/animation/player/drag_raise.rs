use std::path::{Path, PathBuf};

use crate::stats_panel::PetMode;

use super::{AnimationPlayer, PinchPlayer, StartupPlayer, TouchPlayer};
use crate::animation::assets::{
    collect_drag_raise_end_variants, collect_drag_raise_loop_files, collect_drag_raise_start_files,
    pseudo_random_index,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum DragPlaybackMode {
    None,
    Start,
    Loop,
    End,
}

pub(crate) struct DragRaisePlayer {
    raise_dynamic_root: PathBuf,
    raise_static_root: PathBuf,
    drag_raise_start_files: Vec<PathBuf>,
    drag_raise_start_index: usize,
    drag_raise_loop_files: Vec<PathBuf>,
    drag_raise_loop_index: usize,
    drag_raise_end_variants: Vec<Vec<PathBuf>>,
    drag_raise_end_files: Vec<PathBuf>,
    drag_raise_end_index: usize,
    playback_mode: DragPlaybackMode,
}

impl DragRaisePlayer {
    pub(crate) fn new(raise_dynamic_root: PathBuf, raise_static_root: PathBuf, mode: PetMode) -> Self {
        let drag_raise_start_files = collect_drag_raise_start_files(Path::new(&raise_static_root), mode);
        let drag_raise_loop_files = collect_drag_raise_loop_files(Path::new(&raise_dynamic_root), mode);
        let drag_raise_end_variants = collect_drag_raise_end_variants(Path::new(&raise_static_root), mode);

        Self {
            raise_dynamic_root,
            raise_static_root,
            drag_raise_start_files,
            drag_raise_start_index: 0,
            drag_raise_loop_files,
            drag_raise_loop_index: 0,
            drag_raise_end_variants,
            drag_raise_end_files: Vec::new(),
            drag_raise_end_index: 0,
            playback_mode: DragPlaybackMode::None,
        }
    }

    pub(crate) fn start(
        &mut self,
        pinch: &mut PinchPlayer,
        touch: &mut TouchPlayer,
        startup: &mut StartupPlayer,
    ) {
        if !self.drag_raise_start_files.is_empty() {
            startup.stop();
            pinch.stop();
            touch.stop();
            self.playback_mode = DragPlaybackMode::Start;
            self.drag_raise_start_index = 0;
        } else if !self.drag_raise_loop_files.is_empty() {
            startup.stop();
            pinch.stop();
            touch.stop();
            self.playback_mode = DragPlaybackMode::Loop;
            self.drag_raise_loop_index = 0;
        }
    }

    pub(crate) fn continue_loop(
        &mut self,
        pinch: &mut PinchPlayer,
        touch: &mut TouchPlayer,
        startup: &mut StartupPlayer,
    ) {
        if self.playback_mode == DragPlaybackMode::Start || self.drag_raise_loop_files.is_empty() {
            return;
        }

        startup.stop();
        pinch.stop();
        touch.stop();

        if self.playback_mode != DragPlaybackMode::Loop {
            self.drag_raise_loop_index = 0;
        }
        self.playback_mode = DragPlaybackMode::Loop;
    }

    pub(crate) fn end(&mut self) {
        if self.drag_raise_end_variants.is_empty() {
            self.playback_mode = DragPlaybackMode::None;
            return;
        }

        let variant_index = pseudo_random_index(self.drag_raise_end_variants.len());
        self.drag_raise_end_files = self.drag_raise_end_variants[variant_index].clone();
        self.drag_raise_end_index = 0;
        self.playback_mode = DragPlaybackMode::End;
    }
}

impl AnimationPlayer for DragRaisePlayer {
    fn is_active(&self) -> bool {
        self.playback_mode != DragPlaybackMode::None
    }

    fn next_frame(&mut self) -> Option<PathBuf> {
        match self.playback_mode {
            DragPlaybackMode::None => None,
            DragPlaybackMode::Start => {
                if self.drag_raise_start_files.is_empty() {
                    self.playback_mode = DragPlaybackMode::None;
                    return None;
                }

                let frame = self.drag_raise_start_files[self.drag_raise_start_index].clone();
                let next = self.drag_raise_start_index + 1;
                if next < self.drag_raise_start_files.len() {
                    self.drag_raise_start_index = next;
                } else if !self.drag_raise_loop_files.is_empty() {
                    self.playback_mode = DragPlaybackMode::Loop;
                    self.drag_raise_loop_index = 0;
                } else {
                    self.playback_mode = DragPlaybackMode::None;
                }
                Some(frame)
            }
            DragPlaybackMode::Loop => {
                if self.drag_raise_loop_files.is_empty() {
                    self.playback_mode = DragPlaybackMode::None;
                    return None;
                }

                let frame = self.drag_raise_loop_files[self.drag_raise_loop_index].clone();
                self.drag_raise_loop_index =
                    (self.drag_raise_loop_index + 1) % self.drag_raise_loop_files.len();
                Some(frame)
            }
            DragPlaybackMode::End => {
                if self.drag_raise_end_files.is_empty() {
                    self.playback_mode = DragPlaybackMode::None;
                    return None;
                }

                let frame = self.drag_raise_end_files[self.drag_raise_end_index].clone();
                let next = self.drag_raise_end_index + 1;
                if next < self.drag_raise_end_files.len() {
                    self.drag_raise_end_index = next;
                } else {
                    self.playback_mode = DragPlaybackMode::None;
                }
                Some(frame)
            }
        }
    }

    fn stop(&mut self) {
        self.playback_mode = DragPlaybackMode::None;
        self.drag_raise_start_index = 0;
        self.drag_raise_loop_index = 0;
        self.drag_raise_end_index = 0;
        self.drag_raise_end_files.clear();
    }

    fn reload(&mut self, mode: PetMode) {
        self.drag_raise_start_files =
            collect_drag_raise_start_files(Path::new(&self.raise_static_root), mode);
        self.drag_raise_start_index = 0;
        self.drag_raise_loop_files =
            collect_drag_raise_loop_files(Path::new(&self.raise_dynamic_root), mode);
        self.drag_raise_loop_index = 0;
        self.drag_raise_end_variants =
            collect_drag_raise_end_variants(Path::new(&self.raise_static_root), mode);
        self.drag_raise_end_files.clear();
        self.drag_raise_end_index = 0;
    }
}
