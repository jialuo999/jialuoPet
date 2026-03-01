use std::path::{Path, PathBuf};

use crate::stats::PetMode;

use super::AnimationPlayer;
use crate::animation::assets::{collect_shutdown_variants, pseudo_random_index};
use crate::animation::requests::set_shutdown_animation_finished;

pub(crate) struct ShutdownPlayer {
    shutdown_root: PathBuf,
    shutdown_variants: Vec<Vec<PathBuf>>,
    shutdown_files: Vec<PathBuf>,
    shutdown_index: usize,
    shutdown_hold_frame: Option<PathBuf>,
    playing_shutdown: bool,
}

impl ShutdownPlayer {
    pub(crate) fn new(shutdown_root: PathBuf, mode: PetMode) -> Self {
        let shutdown_variants = collect_shutdown_variants(Path::new(&shutdown_root), mode);
        Self {
            shutdown_root,
            shutdown_variants,
            shutdown_files: Vec::new(),
            shutdown_index: 0,
            shutdown_hold_frame: None,
            playing_shutdown: false,
        }
    }

    pub(crate) fn start(&mut self) {
        set_shutdown_animation_finished(false);

        if self.shutdown_variants.is_empty() {
            self.playing_shutdown = false;
            set_shutdown_animation_finished(true);
            return;
        }

        let variant_index = pseudo_random_index(self.shutdown_variants.len());
        self.shutdown_files = self.shutdown_variants[variant_index].clone();
        self.shutdown_index = 0;
        self.playing_shutdown = true;
        self.shutdown_hold_frame = self.shutdown_files.first().cloned();
    }
}

impl AnimationPlayer for ShutdownPlayer {
    fn is_active(&self) -> bool {
        self.playing_shutdown || self.shutdown_hold_frame.is_some()
    }

    fn next_frame(&mut self) -> Option<PathBuf> {
        if self.playing_shutdown {
            if self.shutdown_files.is_empty() {
                self.playing_shutdown = false;
                set_shutdown_animation_finished(true);
                return self.shutdown_hold_frame.clone();
            }

            let frame = self.shutdown_files[self.shutdown_index].clone();
            self.shutdown_hold_frame = Some(frame.clone());
            let next = self.shutdown_index + 1;
            if next < self.shutdown_files.len() {
                self.shutdown_index = next;
            } else {
                self.playing_shutdown = false;
                set_shutdown_animation_finished(true);
            }
            return Some(frame);
        }

        self.shutdown_hold_frame.clone()
    }

    fn interrupt(&mut self, skip_to_end: bool) {
        self.playing_shutdown = false;
        self.shutdown_files.clear();
        self.shutdown_index = 0;
        if skip_to_end {
            self.shutdown_hold_frame = None;
        }
    }

    fn reload(&mut self, mode: PetMode) {
        self.shutdown_variants = collect_shutdown_variants(Path::new(&self.shutdown_root), mode);
    }
}
