use std::path::{Path, PathBuf};

use crate::config::CAROUSEL_INTERVAL_MS;
use crate::stats::PetMode;

use super::{AnimationPlayer, PinchPlayer, StartupPlayer, TouchPlayer};
use crate::animation::assets::{
    collect_drag_raise_end_variants, collect_drag_raise_loop_files, collect_drag_raise_start_files,
    collect_drag_raise_static_b_variants, pseudo_random_index,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum DragPlaybackMode {
    None,
    DynamicLoop,
    StaticStart,
    StaticLoop,
    End,
}

const DRAG_STATIC_TRIGGER_MS: u64 = 20_000;
const DRAG_STATIC_B_LOOP_MIN: u8 = 3;
const DRAG_STATIC_B_LOOP_MAX: u8 = 7;
const DRAG_STATIC_B_FRAME_HOLD_TICKS: u8 = 1;

pub(crate) struct DragRaisePlayer {
    raise_dynamic_root: PathBuf,
    raise_static_root: PathBuf,
    drag_raise_static_start_files: Vec<PathBuf>,
    drag_raise_static_start_index: usize,
    drag_raise_loop_files: Vec<PathBuf>,
    drag_raise_loop_index: usize,
    drag_raise_static_b_variants: Vec<Vec<PathBuf>>,
    drag_raise_static_b_files: Vec<PathBuf>,
    drag_raise_static_b_index: usize,
    drag_raise_static_b_remaining_loops: u8,
    drag_raise_static_b_frame_hold_ticks: u8,
    drag_raise_static_b_last_frame: Option<PathBuf>,
    dynamic_drag_ticks: u32,
    drag_raise_end_variants: Vec<Vec<PathBuf>>,
    drag_raise_end_files: Vec<PathBuf>,
    drag_raise_end_index: usize,
    playback_mode: DragPlaybackMode,
}

impl DragRaisePlayer {
    pub(crate) fn new(raise_dynamic_root: PathBuf, raise_static_root: PathBuf, mode: PetMode) -> Self {
        let drag_raise_static_start_files =
            collect_drag_raise_start_files(Path::new(&raise_static_root), mode);
        let drag_raise_loop_files = collect_drag_raise_loop_files(Path::new(&raise_dynamic_root), mode);
        let drag_raise_static_b_variants =
            collect_drag_raise_static_b_variants(Path::new(&raise_static_root), mode);
        let drag_raise_end_variants = collect_drag_raise_end_variants(Path::new(&raise_static_root), mode);

        Self {
            raise_dynamic_root,
            raise_static_root,
            drag_raise_static_start_files,
            drag_raise_static_start_index: 0,
            drag_raise_loop_files,
            drag_raise_loop_index: 0,
            drag_raise_static_b_variants,
            drag_raise_static_b_files: Vec::new(),
            drag_raise_static_b_index: 0,
            drag_raise_static_b_remaining_loops: 0,
            drag_raise_static_b_frame_hold_ticks: 0,
            drag_raise_static_b_last_frame: None,
            dynamic_drag_ticks: 0,
            drag_raise_end_variants,
            drag_raise_end_files: Vec::new(),
            drag_raise_end_index: 0,
            playback_mode: DragPlaybackMode::None,
        }
    }

    fn drag_static_trigger_ticks() -> u32 {
        ((DRAG_STATIC_TRIGGER_MS + CAROUSEL_INTERVAL_MS - 1) / CAROUSEL_INTERVAL_MS) as u32
    }

    fn choose_static_b_loop_count() -> u8 {
        let span = (DRAG_STATIC_B_LOOP_MAX - DRAG_STATIC_B_LOOP_MIN + 1) as usize;
        DRAG_STATIC_B_LOOP_MIN + pseudo_random_index(span) as u8
    }

    fn prepare_static_b_cycle(&mut self) -> bool {
        if self.drag_raise_static_b_variants.is_empty() {
            return false;
        }

        let variant_index = pseudo_random_index(self.drag_raise_static_b_variants.len());
        self.drag_raise_static_b_files = self.drag_raise_static_b_variants[variant_index].clone();
        self.drag_raise_static_b_index = 0;
        self.drag_raise_static_b_remaining_loops = Self::choose_static_b_loop_count();
        self.drag_raise_static_b_frame_hold_ticks = 0;
        self.drag_raise_static_b_last_frame = None;
        !self.drag_raise_static_b_files.is_empty() && self.drag_raise_static_b_remaining_loops > 0
    }

    fn maybe_trigger_static_cycle(&mut self) {
        if self.playback_mode != DragPlaybackMode::DynamicLoop {
            return;
        }

        self.dynamic_drag_ticks = self.dynamic_drag_ticks.saturating_add(1);
        if self.dynamic_drag_ticks < Self::drag_static_trigger_ticks() {
            return;
        }

        self.dynamic_drag_ticks = 0;

        if !self.drag_raise_static_start_files.is_empty() {
            self.playback_mode = DragPlaybackMode::StaticStart;
            self.drag_raise_static_start_index = 0;
            return;
        }

        if self.prepare_static_b_cycle() {
            self.playback_mode = DragPlaybackMode::StaticLoop;
        }
    }

    pub(crate) fn start(
        &mut self,
        pinch: &mut PinchPlayer,
        touch: &mut TouchPlayer,
        startup: &mut StartupPlayer,
    ) {
        if !self.drag_raise_loop_files.is_empty() {
            startup.stop();
            pinch.stop();
            touch.stop();
            self.playback_mode = DragPlaybackMode::DynamicLoop;
            self.drag_raise_loop_index = 0;
            self.dynamic_drag_ticks = 0;
        }
    }

    pub(crate) fn continue_loop(
        &mut self,
        pinch: &mut PinchPlayer,
        touch: &mut TouchPlayer,
        startup: &mut StartupPlayer,
    ) {
        if self.drag_raise_loop_files.is_empty() {
            return;
        }

        if matches!(
            self.playback_mode,
            DragPlaybackMode::StaticStart | DragPlaybackMode::StaticLoop
        ) {
            return;
        }

        startup.stop();
        pinch.stop();
        touch.stop();

        if self.playback_mode != DragPlaybackMode::DynamicLoop {
            self.drag_raise_loop_index = 0;
            self.dynamic_drag_ticks = 0;
        }
        self.playback_mode = DragPlaybackMode::DynamicLoop;
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

    pub(crate) fn is_playing_end(&self) -> bool {
        self.playback_mode == DragPlaybackMode::End
    }
}

impl AnimationPlayer for DragRaisePlayer {
    fn is_active(&self) -> bool {
        self.playback_mode != DragPlaybackMode::None
    }

    fn next_frame(&mut self) -> Option<PathBuf> {
        match self.playback_mode {
            DragPlaybackMode::None => None,
            DragPlaybackMode::DynamicLoop => {
                if self.drag_raise_loop_files.is_empty() {
                    self.playback_mode = DragPlaybackMode::None;
                    return None;
                }

                let frame = self.drag_raise_loop_files[self.drag_raise_loop_index].clone();
                self.drag_raise_loop_index =
                    (self.drag_raise_loop_index + 1) % self.drag_raise_loop_files.len();
                self.maybe_trigger_static_cycle();
                Some(frame)
            }
            DragPlaybackMode::StaticStart => {
                if self.drag_raise_static_start_files.is_empty() {
                    if self.prepare_static_b_cycle() {
                        self.playback_mode = DragPlaybackMode::StaticLoop;
                        return self.next_frame();
                    }
                    self.playback_mode = DragPlaybackMode::DynamicLoop;
                    self.drag_raise_loop_index = 0;
                    self.dynamic_drag_ticks = 0;
                    return self.next_frame();
                }

                let frame = self.drag_raise_static_start_files[self.drag_raise_static_start_index].clone();
                let next = self.drag_raise_static_start_index + 1;
                if next < self.drag_raise_static_start_files.len() {
                    self.drag_raise_static_start_index = next;
                } else if self.prepare_static_b_cycle() {
                    self.playback_mode = DragPlaybackMode::StaticLoop;
                } else {
                    self.playback_mode = DragPlaybackMode::DynamicLoop;
                    self.drag_raise_loop_index = 0;
                    self.dynamic_drag_ticks = 0;
                }
                Some(frame)
            }
            DragPlaybackMode::StaticLoop => {
                if self.drag_raise_static_b_files.is_empty() || self.drag_raise_static_b_remaining_loops == 0 {
                    self.playback_mode = DragPlaybackMode::DynamicLoop;
                    self.drag_raise_loop_index = 0;
                    self.dynamic_drag_ticks = 0;
                    self.drag_raise_static_b_frame_hold_ticks = 0;
                    self.drag_raise_static_b_last_frame = None;
                    return self.next_frame();
                }

                if self.drag_raise_static_b_frame_hold_ticks > 0 {
                    self.drag_raise_static_b_frame_hold_ticks -= 1;
                    return self.drag_raise_static_b_last_frame.clone();
                }

                let frame = self.drag_raise_static_b_files[self.drag_raise_static_b_index].clone();
                let next = self.drag_raise_static_b_index + 1;
                if next < self.drag_raise_static_b_files.len() {
                    self.drag_raise_static_b_index = next;
                } else if self.drag_raise_static_b_remaining_loops > 1 {
                    self.drag_raise_static_b_remaining_loops -= 1;
                    self.drag_raise_static_b_index = 0;
                } else {
                    self.drag_raise_static_b_remaining_loops = 0;
                    self.drag_raise_static_b_index = 0;
                    self.drag_raise_static_b_files.clear();
                    self.playback_mode = DragPlaybackMode::DynamicLoop;
                    self.drag_raise_loop_index = 0;
                    self.dynamic_drag_ticks = 0;
                }
                self.drag_raise_static_b_last_frame = Some(frame.clone());
                self.drag_raise_static_b_frame_hold_ticks = DRAG_STATIC_B_FRAME_HOLD_TICKS;
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

    fn interrupt(&mut self, skip_to_end: bool) {
        if !skip_to_end
            && (self.playback_mode == DragPlaybackMode::DynamicLoop
                || self.playback_mode == DragPlaybackMode::StaticStart
                || self.playback_mode == DragPlaybackMode::StaticLoop)
            && !self.drag_raise_end_variants.is_empty()
        {
            let variant_index = pseudo_random_index(self.drag_raise_end_variants.len());
            self.drag_raise_end_files = self.drag_raise_end_variants[variant_index].clone();
            self.drag_raise_end_index = 0;
            self.playback_mode = DragPlaybackMode::End;
            return;
        }

        self.playback_mode = DragPlaybackMode::None;
        self.drag_raise_static_start_index = 0;
        self.drag_raise_loop_index = 0;
        self.drag_raise_static_b_index = 0;
        self.drag_raise_static_b_remaining_loops = 0;
        self.drag_raise_static_b_frame_hold_ticks = 0;
        self.drag_raise_static_b_last_frame = None;
        self.dynamic_drag_ticks = 0;
        self.drag_raise_end_index = 0;
        self.drag_raise_static_b_files.clear();
        self.drag_raise_end_files.clear();
    }

    fn reload(&mut self, mode: PetMode) {
        self.drag_raise_static_start_files =
            collect_drag_raise_start_files(Path::new(&self.raise_static_root), mode);
        self.drag_raise_static_start_index = 0;
        self.drag_raise_loop_files =
            collect_drag_raise_loop_files(Path::new(&self.raise_dynamic_root), mode);
        self.drag_raise_loop_index = 0;
        self.drag_raise_static_b_variants =
            collect_drag_raise_static_b_variants(Path::new(&self.raise_static_root), mode);
        self.drag_raise_static_b_files.clear();
        self.drag_raise_static_b_index = 0;
        self.drag_raise_static_b_remaining_loops = 0;
        self.drag_raise_static_b_frame_hold_ticks = 0;
        self.drag_raise_static_b_last_frame = None;
        self.dynamic_drag_ticks = 0;
        self.drag_raise_end_variants =
            collect_drag_raise_end_variants(Path::new(&self.raise_static_root), mode);
        self.drag_raise_end_files.clear();
        self.drag_raise_end_index = 0;
    }
}
