use std::path::{Path, PathBuf};

use crate::stats::PetMode;

use super::common::{
    collect_dir_paths, collect_png_files, dir_name_matches_mode, load_frames_with_fallback,
    pseudo_random_index, Segment,
};

fn collect_mode_dir(side_hide_root: &Path, mode: PetMode) -> Option<PathBuf> {
    let mut mode_dirs: Vec<PathBuf> = collect_dir_paths(side_hide_root)
        .into_iter()
        .filter(|path| {
            path.file_name()
                .and_then(|s| s.to_str())
                .map(|name| dir_name_matches_mode(name, mode))
                .unwrap_or(false)
        })
        .collect();

    mode_dirs.sort();
    if let Some(path) = mode_dirs.into_iter().next() {
        return Some(path);
    }

    if mode != PetMode::Nomal {
        let mut nomal_dirs: Vec<PathBuf> = collect_dir_paths(side_hide_root)
            .into_iter()
            .filter(|path| {
                path.file_name()
                    .and_then(|s| s.to_str())
                    .map(|name| dir_name_matches_mode(name, PetMode::Nomal))
                    .unwrap_or(false)
            })
            .collect();
        nomal_dirs.sort();
        if let Some(path) = nomal_dirs.into_iter().next() {
            return Some(path);
        }
    }

    if mode != PetMode::Happy {
        let mut happy_dirs: Vec<PathBuf> = collect_dir_paths(side_hide_root)
            .into_iter()
            .filter(|path| {
                path.file_name()
                    .and_then(|s| s.to_str())
                    .map(|name| dir_name_matches_mode(name, PetMode::Happy))
                    .unwrap_or(false)
            })
            .collect();
        happy_dirs.sort();
        return happy_dirs.into_iter().next();
    }

    None
}

pub(crate) fn collect_side_hide_start_files(side_hide_root: &Path, mode: PetMode) -> Vec<PathBuf> {
    let files = load_frames_with_fallback(side_hide_root, mode, Segment::A);
    if !files.is_empty() {
        return files;
    }

    let Some(mode_dir) = collect_mode_dir(side_hide_root, mode) else {
        return Vec::new();
    };

    let mut candidates: Vec<Vec<PathBuf>> = collect_dir_paths(&mode_dir)
        .into_iter()
        .filter(|path| {
            path.file_name()
                .and_then(|s| s.to_str())
                .map(|name| name.eq_ignore_ascii_case("A"))
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
        .collect();

    if candidates.is_empty() {
        Vec::new()
    } else {
        candidates.swap_remove(pseudo_random_index(candidates.len()))
    }
}

pub(crate) fn collect_side_hide_loop_variants(side_hide_root: &Path, mode: PetMode) -> Vec<Vec<PathBuf>> {
    let Some(mode_dir) = collect_mode_dir(side_hide_root, mode) else {
        return Vec::new();
    };

    let mut variants: Vec<(String, Vec<PathBuf>)> = collect_dir_paths(&mode_dir)
        .into_iter()
        .filter_map(|path| {
            let name = path
                .file_name()
                .and_then(|s| s.to_str())
                .map(|value| value.to_string())?;
            if !name.to_ascii_lowercase().starts_with("b") {
                return None;
            }

            let files = collect_png_files(&path).ok()?;
            if files.is_empty() {
                None
            } else {
                Some((name, files))
            }
        })
        .collect();

    variants.sort_by(|left, right| left.0.cmp(&right.0));
    variants.into_iter().map(|(_, files)| files).collect()
}

pub(crate) fn collect_side_hide_end_files(side_hide_root: &Path, mode: PetMode) -> Vec<PathBuf> {
    let files = load_frames_with_fallback(side_hide_root, mode, Segment::C);
    if !files.is_empty() {
        return files;
    }

    let Some(mode_dir) = collect_mode_dir(side_hide_root, mode) else {
        return Vec::new();
    };

    let mut candidates: Vec<Vec<PathBuf>> = collect_dir_paths(&mode_dir)
        .into_iter()
        .filter(|path| {
            path.file_name()
                .and_then(|s| s.to_str())
                .map(|name| name.eq_ignore_ascii_case("C"))
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
        .collect();

    if candidates.is_empty() {
        Vec::new()
    } else {
        candidates.swap_remove(pseudo_random_index(candidates.len()))
    }
}