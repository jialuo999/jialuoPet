use std::path::PathBuf;

use crate::config::AnimationPathConfig;
use crate::stats::PetMode;

use super::AnimationPlayer;
use crate::animation::assets::{
    collect_default_happy_idle_variants, collect_default_mode_idle_variants,
    select_default_files_for_mode,
};

pub(crate) struct DefaultIdlePlayer {
    config: AnimationPathConfig,
    current_mode: PetMode,
    default_happy_variants: Vec<Vec<PathBuf>>,
    default_nomal_variants: Vec<Vec<PathBuf>>,
    default_poor_condition_variants: Vec<Vec<PathBuf>>,
    default_ill_variants: Vec<Vec<PathBuf>>,
    default_files: Vec<PathBuf>,
    default_index: usize,
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

        Ok(Self {
            config: config.clone(),
            current_mode: mode,
            default_happy_variants,
            default_nomal_variants,
            default_poor_condition_variants,
            default_ill_variants,
            default_files,
            default_index: 0,
        })
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
}

impl AnimationPlayer for DefaultIdlePlayer {
    fn is_active(&self) -> bool {
        true
    }

    fn next_frame(&mut self) -> Option<PathBuf> {
        self.next_default_frame()
    }

    fn interrupt(&mut self, _skip_to_end: bool) {
        self.default_index = 0;
    }

    fn reload(&mut self, mode: PetMode) {
        self.current_mode = mode;
        self.default_happy_variants =
            collect_default_happy_idle_variants(&self.config).unwrap_or_default();
        self.default_nomal_variants = collect_default_mode_idle_variants(&self.config, PetMode::Nomal);
        self.default_poor_condition_variants =
            collect_default_mode_idle_variants(&self.config, PetMode::PoorCondition);
        self.default_ill_variants = collect_default_mode_idle_variants(&self.config, PetMode::Ill);
        self.refresh_selection();
    }
}
