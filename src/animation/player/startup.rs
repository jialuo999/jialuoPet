use std::path::{Path, PathBuf};

use crate::stats_panel::PetMode;

use super::AnimationPlayer;
use crate::animation::assets::choose_startup_animation_files;

pub(crate) struct StartupPlayer {
    startup_root: PathBuf,
    files: Vec<PathBuf>,
    index: usize,
    active: bool,
}

impl StartupPlayer {
    pub(crate) fn new(startup_root: PathBuf, mode: PetMode) -> Self {
        let files = choose_startup_animation_files(Path::new(&startup_root), mode).unwrap_or_default();
        let active = !files.is_empty();
        Self {
            startup_root,
            files,
            index: 0,
            active,
        }
    }

    pub(crate) fn peek_first_frame(&self) -> Option<PathBuf> {
        self.files.first().cloned()
    }
}

impl AnimationPlayer for StartupPlayer {
    fn is_active(&self) -> bool {
        self.active && !self.files.is_empty()
    }

    fn next_frame(&mut self) -> Option<PathBuf> {
        if !self.is_active() {
            return None;
        }

        let frame = self.files[self.index].clone();
        let next = self.index + 1;
        if next < self.files.len() {
            self.index = next;
        } else {
            self.active = false;
        }
        Some(frame)
    }

    fn stop(&mut self) {
        self.active = false;
        self.files.clear();
        self.index = 0;
    }

    fn reload(&mut self, _mode: PetMode) {
        let _ = &self.startup_root;
    }
}
