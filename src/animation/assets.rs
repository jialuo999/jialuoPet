use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::stats_panel::PetMode;

pub(super) fn body_asset_path(root: &str, relative: &str) -> PathBuf {
    PathBuf::from(root).join(relative)
}

pub(super) fn pseudo_random_index(len: usize) -> usize {
    if len == 0 {
        return 0;
    }

    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as usize;
    seed % len
}

pub(super) fn collect_png_files(asset_dir: &Path) -> Result<Vec<PathBuf>, String> {
    if !asset_dir.is_dir() {
        return Err(format!("目录不存在：{}", asset_dir.display()));
    }

    let mut image_files: Vec<PathBuf> = fs::read_dir(asset_dir)
        .map_err(|e| format!("无法读取目录 {}: {}", asset_dir.display(), e))?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("png") {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    image_files.sort();
    Ok(image_files)
}

pub(super) fn collect_png_files_recursive_filtered(
    asset_dir: &Path,
    excluded_dirs: &[&str],
) -> Result<Vec<PathBuf>, String> {
    if !asset_dir.is_dir() {
        return Err(format!("目录不存在：{}", asset_dir.display()));
    }

    fn visit_dir(
        current_dir: &Path,
        excluded_dirs: &[&str],
        output: &mut Vec<PathBuf>,
    ) -> Result<(), String> {
        let entries = fs::read_dir(current_dir)
            .map_err(|e| format!("无法读取目录 {}: {}", current_dir.display(), e))?;

        for entry in entries {
            let entry = match entry {
                Ok(value) => value,
                Err(_) => continue,
            };
            let path = entry.path();

            if path.is_dir() {
                let dir_name = match path.file_name().and_then(|s| s.to_str()) {
                    Some(value) => value,
                    None => continue,
                };
                if excluded_dirs
                    .iter()
                    .any(|excluded| dir_name.eq_ignore_ascii_case(excluded))
                {
                    continue;
                }
                visit_dir(&path, excluded_dirs, output)?;
            } else if path.extension().and_then(|s| s.to_str()) == Some("png") {
                output.push(path);
            }
        }

        Ok(())
    }

    let mut image_files = Vec::new();
    visit_dir(asset_dir, excluded_dirs, &mut image_files)?;
    image_files.sort();
    Ok(image_files)
}

fn mode_keyword(mode: PetMode) -> &'static str {
    match mode {
        PetMode::Happy => "happy",
        PetMode::Nomal => "nomal",
        PetMode::PoorCondition => "poorcondition",
        PetMode::Ill => "ill",
    }
}

pub(super) fn dir_name_matches_mode(dir_name: &str, mode: PetMode) -> bool {
    dir_name.to_ascii_lowercase().contains(mode_keyword(mode))
}

pub(super) fn collect_dir_paths(root: &Path) -> Vec<PathBuf> {
    fs::read_dir(root)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(|entry| entry.ok().map(|item| item.path())))
        .filter(|path| path.is_dir())
        .collect()
}

pub(super) fn collect_png_variant_dirs_recursive(root: &Path) -> Vec<PathBuf> {
    fn visit(current: &Path, output: &mut Vec<PathBuf>) {
        let entries = match fs::read_dir(current) {
            Ok(entries) => entries,
            Err(_) => return,
        };

        let mut has_png = false;
        let mut child_dirs = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                child_dirs.push(path);
            } else if path.extension().and_then(|s| s.to_str()) == Some("png") {
                has_png = true;
            }
        }

        if has_png {
            output.push(current.to_path_buf());
        }

        for child in child_dirs {
            visit(&child, output);
        }
    }

    let mut output = Vec::new();
    if root.is_dir() {
        visit(root, &mut output);
    }
    output
}

pub(super) fn collect_mode_variant_dirs(root: &Path, mode: PetMode) -> Vec<PathBuf> {
    let variant_dirs = collect_png_variant_dirs_recursive(root);
    let mut selected: Vec<PathBuf> = variant_dirs
        .iter()
        .filter(|path| {
            path.file_name()
                .and_then(|s| s.to_str())
                .map(|name| dir_name_matches_mode(name, mode))
                .unwrap_or(false)
        })
        .cloned()
        .collect();

    if selected.is_empty() && mode != PetMode::Happy {
        selected = variant_dirs
            .into_iter()
            .filter(|path| {
                path.file_name()
                    .and_then(|s| s.to_str())
                    .map(|name| dir_name_matches_mode(name, PetMode::Happy))
                    .unwrap_or(false)
            })
            .collect();
    }

    selected
}

pub(super) fn path_matches_mode(path: &Path, mode: PetMode) -> bool {
    path.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .map(|name| dir_name_matches_mode(name, mode))
            .unwrap_or(false)
    })
}

pub(super) fn path_contains_keyword(path: &Path, keyword: &str) -> bool {
    let keyword = keyword.to_ascii_lowercase();
    path.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .map(|name| name.to_ascii_lowercase().contains(&keyword))
            .unwrap_or(false)
    })
}

pub(super) fn path_in_stage_branch(path: &Path, touch_root: &Path, stage_prefix: &str) -> bool {
    let stage_prefix = stage_prefix.to_ascii_lowercase();
    let Ok(relative) = path.strip_prefix(touch_root) else {
        return false;
    };

    relative.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .map(|name| name.to_ascii_lowercase().starts_with(&stage_prefix))
            .unwrap_or(false)
    })
}

pub(super) fn touch_variant_key(path: &Path, touch_root: &Path, stage_prefix: &str) -> Option<String> {
    let stage_prefix = stage_prefix.to_ascii_lowercase();
    let relative = path.strip_prefix(touch_root).ok()?;
    let components: Vec<String> = relative
        .components()
        .filter_map(|component| component.as_os_str().to_str().map(|value| value.to_string()))
        .collect();

    let stage_index = components
        .iter()
        .position(|name| name.to_ascii_lowercase().starts_with(&stage_prefix))?;
    if stage_index + 1 < components.len() {
        Some(components[stage_index + 1].to_ascii_lowercase())
    } else {
        None
    }
}
