use std::fs;
use std::path::{Path, PathBuf};

use crate::config::AnimationPathConfig;
use crate::stats_panel::PetMode;

use super::assets::{
    body_asset_path, collect_dir_paths, collect_mode_variant_dirs, collect_png_files,
    collect_png_files_recursive_filtered, collect_png_variant_dirs_recursive,
    dir_name_matches_mode, pseudo_random_index,
};

pub(super) fn choose_startup_animation_files(startup_root: &Path, mode: PetMode) -> Option<Vec<PathBuf>> {
    let startup_dirs: Vec<PathBuf> = fs::read_dir(startup_root)
        .ok()?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if !path.is_dir() {
                return None;
            }

            let dir_name = path.file_name()?.to_str()?;
            let include_dir = if mode == PetMode::Happy {
                dir_name_matches_mode(dir_name, PetMode::Happy) || dir_name.eq_ignore_ascii_case("26new")
            } else {
                dir_name_matches_mode(dir_name, mode)
            };
            if !include_dir {
                return None;
            }

            Some(path)
        })
        .collect();

    if startup_dirs.is_empty() {
        return None;
    }

    let mut available_variants: Vec<Vec<PathBuf>> = startup_dirs
        .iter()
        .filter_map(|dir| {
            let files = collect_png_files(dir).ok()?;
            if files.is_empty() {
                None
            } else {
                Some(files)
            }
        })
        .collect();

    if available_variants.is_empty() {
        return None;
    }

    let selected_index = pseudo_random_index(available_variants.len());
    Some(available_variants.swap_remove(selected_index))
}

pub(super) fn collect_drag_raise_loop_files(raise_dynamic_root: &Path, mode: PetMode) -> Vec<PathBuf> {
    let mut mode_dirs: Vec<PathBuf> = collect_dir_paths(raise_dynamic_root)
        .into_iter()
        .filter(|path| {
            path.file_name()
                .and_then(|s| s.to_str())
                .map(|name| dir_name_matches_mode(name, mode))
                .unwrap_or(false)
        })
        .collect();

    if mode_dirs.is_empty() && mode != PetMode::Happy {
        mode_dirs = collect_dir_paths(raise_dynamic_root)
            .into_iter()
            .filter(|path| {
                path.file_name()
                    .and_then(|s| s.to_str())
                    .map(|name| dir_name_matches_mode(name, PetMode::Happy))
                    .unwrap_or(false)
            })
            .collect();
    }

    if mode_dirs.is_empty() {
        return Vec::new();
    }

    let selected_index = pseudo_random_index(mode_dirs.len());
    collect_png_files_recursive_filtered(&mode_dirs[selected_index], &[]).unwrap_or_default()
}

pub(super) fn collect_drag_raise_start_files(_raise_static_root: &Path, _mode: PetMode) -> Vec<PathBuf> {
    Vec::new()
}

pub(super) fn collect_drag_raise_end_variants(raise_static_root: &Path, mode: PetMode) -> Vec<Vec<PathBuf>> {
    let mut mode_dirs: Vec<PathBuf> = collect_dir_paths(raise_static_root)
        .iter()
        .filter(|path| {
            let dir_name = path.file_name().and_then(|s| s.to_str()).unwrap_or_default();
            dir_name.to_ascii_lowercase().starts_with("c_") && dir_name_matches_mode(dir_name, mode)
        })
        .cloned()
        .collect();

    if mode_dirs.is_empty() && mode != PetMode::Happy {
        mode_dirs = collect_dir_paths(raise_static_root)
            .iter()
            .filter(|path| {
                let dir_name = path.file_name().and_then(|s| s.to_str()).unwrap_or_default();
                dir_name.to_ascii_lowercase().starts_with("c_")
                    && dir_name_matches_mode(dir_name, PetMode::Happy)
            })
            .cloned()
            .collect();
    }

    mode_dirs
        .iter()
        .filter_map(|dir| {
            let files = collect_png_files(dir).ok()?;
            if files.is_empty() {
                None
            } else {
                Some(files)
            }
        })
        .collect()
}

fn collect_pinch_stage_variants(
    pinch_root: &Path,
    mode: PetMode,
    stage_prefix: &str,
) -> Vec<Vec<PathBuf>> {
    let mode_dirs: Vec<PathBuf> = collect_dir_paths(pinch_root)
        .iter()
        .filter(|path| {
            path.file_name()
                .and_then(|s| s.to_str())
                .map(|name| dir_name_matches_mode(name, mode))
                .unwrap_or(false)
        })
        .cloned()
        .collect();

    let selected_mode_dir = if mode_dirs.is_empty() && mode != PetMode::Happy {
        collect_dir_paths(pinch_root)
            .into_iter()
            .find(|path| {
                path.file_name()
                    .and_then(|s| s.to_str())
                    .map(|name| dir_name_matches_mode(name, PetMode::Happy))
                    .unwrap_or(false)
            })
    } else {
        mode_dirs.into_iter().next()
    };

    let Some(mode_dir) = selected_mode_dir else {
        return Vec::new();
    };

    collect_dir_paths(&mode_dir)
        .into_iter()
        .filter(|path| {
            path.file_name()
                .and_then(|s| s.to_str())
                .map(|name| name.to_ascii_lowercase().starts_with(&stage_prefix.to_ascii_lowercase()))
                .unwrap_or(false)
        })
        .filter_map(|path| {
            let files = collect_png_files(&path).ok()?;
            if files.is_empty() {
                None
            } else {
                Some(files)
            }
        })
        .collect()
}

pub(super) fn collect_pinch_start_files(pinch_root: &Path, mode: PetMode) -> Vec<PathBuf> {
    let mut variants = collect_pinch_stage_variants(pinch_root, mode, "A");
    if variants.is_empty() {
        Vec::new()
    } else {
        variants.swap_remove(pseudo_random_index(variants.len()))
    }
}

pub(super) fn collect_pinch_loop_variants(pinch_root: &Path, mode: PetMode) -> Vec<Vec<PathBuf>> {
    collect_pinch_stage_variants(pinch_root, mode, "B")
}

pub(super) fn collect_pinch_end_files(pinch_root: &Path, mode: PetMode) -> Vec<PathBuf> {
    let mut variants = collect_pinch_stage_variants(pinch_root, mode, "C");
    if variants.is_empty() {
        Vec::new()
    } else {
        variants.swap_remove(pseudo_random_index(variants.len()))
    }
}

pub(super) fn collect_shutdown_variants(shutdown_root: &Path, mode: PetMode) -> Vec<Vec<PathBuf>> {
    collect_mode_variant_dirs(shutdown_root, mode)
        .iter()
        .filter_map(|dir| {
            let files = collect_png_files(dir).ok()?;
            if files.is_empty() {
                None
            } else {
                Some(files)
            }
        })
        .collect()
}

pub(super) fn collect_default_happy_idle_variants(
    animation_config: &AnimationPathConfig,
) -> Result<Vec<Vec<PathBuf>>, String> {
    let mut variants = Vec::new();
    for variant in &animation_config.default_happy_idle_variants {
        let dir = body_asset_path(&animation_config.assets_body_root, variant);
        let files = collect_png_files(&dir)?;
        if !files.is_empty() {
            variants.push(files);
        }
    }
    Ok(variants)
}

pub(super) fn collect_default_mode_idle_variants(
    animation_config: &AnimationPathConfig,
    mode: PetMode,
) -> Vec<Vec<PathBuf>> {
    if mode == PetMode::Happy {
        return collect_default_happy_idle_variants(animation_config).unwrap_or_default();
    }

    let root = match mode {
        PetMode::Nomal => body_asset_path(
            &animation_config.assets_body_root,
            &animation_config.default_nomal_idle_root,
        ),
        PetMode::PoorCondition => body_asset_path(
            &animation_config.assets_body_root,
            &animation_config.default_poor_condition_idle_root,
        ),
        PetMode::Ill => body_asset_path(
            &animation_config.assets_body_root,
            &animation_config.default_ill_idle_root,
        ),
        PetMode::Happy => unreachable!(),
    };

    collect_png_variant_dirs_recursive(&root)
        .iter()
        .filter_map(|dir| {
            let files = collect_png_files(dir).ok()?;
            if files.is_empty() {
                None
            } else {
                Some(files)
            }
        })
        .collect()
}

fn pick_random_variant(variants: &[Vec<PathBuf>]) -> Option<Vec<PathBuf>> {
    if variants.is_empty() {
        None
    } else {
        Some(variants[pseudo_random_index(variants.len())].clone())
    }
}

pub(super) fn select_default_files_for_mode(
    mode: PetMode,
    happy_variants: &[Vec<PathBuf>],
    nomal_variants: &[Vec<PathBuf>],
    poor_condition_variants: &[Vec<PathBuf>],
    ill_variants: &[Vec<PathBuf>],
) -> Vec<PathBuf> {
    let selected = match mode {
        PetMode::Happy => pick_random_variant(happy_variants),
        PetMode::Nomal => pick_random_variant(nomal_variants),
        PetMode::PoorCondition => pick_random_variant(poor_condition_variants),
        PetMode::Ill => pick_random_variant(ill_variants),
    };

    selected
        .or_else(|| pick_random_variant(happy_variants))
        .unwrap_or_default()
}
