use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::animation::assets::collect_png_files_recursive_for_mode;
use crate::stats::PetMode;

use super::{AnimationPlayer, StartupPlayer};

#[derive(Clone, Copy, PartialEq, Eq)]
enum StudyAction {
    None,
    Book,
    Paint,
    Research,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum StudyPlaybackStage {
    None,
    Start,
    Loop,
    End,
}

struct StudySegments {
    start_files: Vec<PathBuf>,
    loop_variants: Vec<Vec<PathBuf>>,
    end_files: Vec<PathBuf>,
}

pub(crate) struct StudyPlayer {
    study_book_root: PathBuf,
    study_paint_root: PathBuf,
    study_research_root: PathBuf,
    mode: PetMode,
    action: StudyAction,
    stage: StudyPlaybackStage,
    start_files: Vec<PathBuf>,
    start_index: usize,
    loop_variants: Vec<Vec<PathBuf>>,
    loop_variant_index: usize,
    loop_files: Vec<PathBuf>,
    loop_index: usize,
    end_files: Vec<PathBuf>,
    end_index: usize,
    loop_deadline: Option<Instant>,
    pending_resume_after_drag: Option<(StudyAction, u64)>,
}

impl StudyPlayer {
    pub(crate) fn new(
        study_book_root: PathBuf,
        study_paint_root: PathBuf,
        study_research_root: PathBuf,
        mode: PetMode,
    ) -> Self {
        Self {
            study_book_root,
            study_paint_root,
            study_research_root,
            mode,
            action: StudyAction::None,
            stage: StudyPlaybackStage::None,
            start_files: Vec::new(),
            start_index: 0,
            loop_variants: Vec::new(),
            loop_variant_index: 0,
            loop_files: Vec::new(),
            loop_index: 0,
            end_files: Vec::new(),
            end_index: 0,
            loop_deadline: None,
            pending_resume_after_drag: None,
        }
    }

    fn root_for_action(&self, action: StudyAction) -> PathBuf {
        match action {
            StudyAction::Book => self.study_book_root.clone(),
            StudyAction::Paint => self.study_paint_root.clone(),
            StudyAction::Research => self.study_research_root.clone(),
            StudyAction::None => PathBuf::new(),
        }
    }

    fn clear_playback_state(&mut self) {
        self.action = StudyAction::None;
        self.stage = StudyPlaybackStage::None;
        self.start_files.clear();
        self.start_index = 0;
        self.loop_variants.clear();
        self.loop_variant_index = 0;
        self.loop_files.clear();
        self.loop_index = 0;
        self.end_files.clear();
        self.end_index = 0;
        self.loop_deadline = None;
    }

    fn component_matches_segment(name: &str, stage_prefix: &str) -> bool {
        let normalized = name.to_ascii_lowercase();
        let stage = stage_prefix.to_ascii_lowercase();

        if normalized == stage {
            return true;
        }

        normalized
            .strip_prefix(&stage)
            .and_then(|rest| rest.chars().next())
            .map(|next| !next.is_ascii_alphanumeric())
            .unwrap_or(false)
    }

    fn detect_segment(root: &Path, file: &Path) -> Option<char> {
        let relative = file.strip_prefix(root).ok()?;
        let parent = relative.parent()?;
        for component in parent.components() {
            let name = component.as_os_str().to_str()?;
            if Self::component_matches_segment(name, "a") {
                return Some('A');
            }
            if Self::component_matches_segment(name, "b") {
                return Some('B');
            }
            if Self::component_matches_segment(name, "c") {
                return Some('C');
            }
        }
        None
    }

    fn collect_segments(root: &Path, mode: PetMode) -> StudySegments {
        let files = collect_png_files_recursive_for_mode(root, mode);
        let mut start_files = Vec::new();
        let mut end_files = Vec::new();
        let mut loop_variant_map: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
        let mut fallback_loop_files = Vec::new();

        for file in files {
            match Self::detect_segment(root, &file) {
                Some('A') => start_files.push(file),
                Some('C') => end_files.push(file),
                Some('B') => {
                    let key = file
                        .parent()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|| "__b__".to_string());
                    loop_variant_map.entry(key).or_default().push(file);
                }
                _ => fallback_loop_files.push(file),
            }
        }

        start_files.sort();
        end_files.sort();

        let mut loop_variants: Vec<Vec<PathBuf>> = loop_variant_map
            .into_values()
            .filter_map(|mut variant| {
                variant.sort();
                if variant.is_empty() {
                    None
                } else {
                    Some(variant)
                }
            })
            .collect();

        fallback_loop_files.sort();
        if !fallback_loop_files.is_empty() {
            loop_variants.push(fallback_loop_files);
        }

        StudySegments {
            start_files,
            loop_variants,
            end_files,
        }
    }

    fn begin_loop_stage(&mut self, duration_secs: u64) {
        if self.loop_variants.is_empty() {
            self.stage = StudyPlaybackStage::None;
            return;
        }

        self.loop_variant_index = 0;
        self.loop_files = self.loop_variants[self.loop_variant_index].clone();
        self.loop_index = 0;
        self.stage = StudyPlaybackStage::Loop;
        self.loop_deadline = Some(Instant::now() + Duration::from_secs(duration_secs.max(1)));
    }

    fn start_with_root(
        &mut self,
        root: &Path,
        action: StudyAction,
        duration_secs: u64,
        startup: &mut StartupPlayer,
    ) {
        let segments = Self::collect_segments(root, self.mode);
        if segments.start_files.is_empty() && segments.loop_variants.is_empty() && segments.end_files.is_empty() {
            return;
        }

        startup.stop();
        self.action = action;
        self.start_files = segments.start_files;
        self.loop_variants = segments.loop_variants;
        self.end_files = segments.end_files;
        self.start_index = 0;
        self.loop_variant_index = 0;
        self.loop_index = 0;
        self.end_index = 0;
        self.loop_files.clear();

        if !self.start_files.is_empty() {
            self.stage = StudyPlaybackStage::Start;
            self.loop_deadline = Some(Instant::now() + Duration::from_secs(duration_secs.max(1)));
        } else if !self.loop_variants.is_empty() {
            self.begin_loop_stage(duration_secs);
        } else if !self.end_files.is_empty() {
            self.stage = StudyPlaybackStage::End;
            self.loop_deadline = None;
        } else {
            self.stage = StudyPlaybackStage::None;
            self.loop_deadline = None;
        }
    }

    pub(crate) fn start_book(&mut self, startup: &mut StartupPlayer, duration_secs: u64) {
        let root = self.study_book_root.clone();
        self.pending_resume_after_drag = None;
        self.start_with_root(&root, StudyAction::Book, duration_secs, startup);
    }

    pub(crate) fn start_paint(&mut self, startup: &mut StartupPlayer, duration_secs: u64) {
        let root = self.study_paint_root.clone();
        self.pending_resume_after_drag = None;
        self.start_with_root(&root, StudyAction::Paint, duration_secs, startup);
    }

    pub(crate) fn start_research(&mut self, startup: &mut StartupPlayer, duration_secs: u64) {
        let root = self.study_research_root.clone();
        self.pending_resume_after_drag = None;
        self.start_with_root(&root, StudyAction::Research, duration_secs, startup);
    }

    pub(crate) fn interrupt_by_drag(&mut self) {
        if !self.is_active() {
            return;
        }

        if matches!(self.stage, StudyPlaybackStage::Start | StudyPlaybackStage::Loop)
            && self.action != StudyAction::None
        {
            let remaining_secs = self
                .loop_deadline
                .and_then(|deadline| deadline.checked_duration_since(Instant::now()))
                .map(|d| d.as_secs().max(1))
                .unwrap_or(1);
            self.pending_resume_after_drag = Some((self.action, remaining_secs));
        }

        self.clear_playback_state();
    }

    pub(crate) fn resume_if_pending_after_drag(&mut self, startup: &mut StartupPlayer) {
        let Some((action, remaining_secs)) = self.pending_resume_after_drag.take() else {
            return;
        };

        let root = self.root_for_action(action);
        if root.as_os_str().is_empty() {
            return;
        }
        self.start_with_root(&root, action, remaining_secs, startup);
    }

    pub(crate) fn clear_pending_resume_after_drag(&mut self) {
        self.pending_resume_after_drag = None;
    }
}

impl AnimationPlayer for StudyPlayer {
    fn is_active(&self) -> bool {
        self.action != StudyAction::None
    }

    fn next_frame(&mut self) -> Option<PathBuf> {
        match self.stage {
            StudyPlaybackStage::None => None,
            StudyPlaybackStage::Start => {
                if self.start_files.is_empty() {
                    self.stage = StudyPlaybackStage::None;
                    self.action = StudyAction::None;
                    return None;
                }

                let frame = self.start_files[self.start_index].clone();
                let next = self.start_index + 1;
                if next < self.start_files.len() {
                    self.start_index = next;
                } else if !self.loop_variants.is_empty() {
                    let remaining_secs = self
                        .loop_deadline
                        .and_then(|deadline| deadline.checked_duration_since(Instant::now()))
                        .map(|d| d.as_secs().max(1))
                        .unwrap_or(1);
                    self.begin_loop_stage(remaining_secs);
                } else if !self.end_files.is_empty() {
                    self.stage = StudyPlaybackStage::End;
                    self.end_index = 0;
                } else {
                    self.stop();
                }
                Some(frame)
            }
            StudyPlaybackStage::Loop => {
                if self.loop_files.is_empty() {
                    if !self.end_files.is_empty() {
                        self.stage = StudyPlaybackStage::End;
                        self.end_index = 0;
                    } else {
                        self.stop();
                    }
                    return None;
                }

                let frame = self.loop_files[self.loop_index].clone();
                let next = self.loop_index + 1;
                if next < self.loop_files.len() {
                    self.loop_index = next;
                } else {
                    let is_last_variant = self.loop_variant_index + 1 >= self.loop_variants.len();
                    if self.loop_variants.len() > 1 {
                        self.loop_variant_index = if is_last_variant {
                            0
                        } else {
                            self.loop_variant_index + 1
                        };
                        self.loop_files = self.loop_variants[self.loop_variant_index].clone();
                    }
                    self.loop_index = 0;
                    let reached_deadline_on_cycle_boundary = is_last_variant
                        && self
                            .loop_deadline
                            .map(|deadline| Instant::now() >= deadline)
                            .unwrap_or(false);
                    if reached_deadline_on_cycle_boundary {
                        if !self.end_files.is_empty() {
                            self.stage = StudyPlaybackStage::End;
                            self.end_index = 0;
                        } else {
                            self.stop();
                        }
                    }
                }
                Some(frame)
            }
            StudyPlaybackStage::End => {
                if self.end_files.is_empty() {
                    self.stop();
                    return None;
                }

                let frame = self.end_files[self.end_index].clone();
                let next = self.end_index + 1;
                if next < self.end_files.len() {
                    self.end_index = next;
                } else {
                    self.stop();
                }
                Some(frame)
            }
        }
    }

    fn interrupt(&mut self, _skip_to_end: bool) {
        if !_skip_to_end
            && matches!(self.stage, StudyPlaybackStage::Start | StudyPlaybackStage::Loop)
            && !self.end_files.is_empty()
        {
            self.stage = StudyPlaybackStage::End;
            self.end_index = 0;
            self.loop_deadline = None;
            return;
        }

        self.clear_playback_state();
        self.pending_resume_after_drag = None;
    }

    fn reload(&mut self, mode: PetMode) {
        self.mode = mode;
        self.interrupt(true);
    }
}
