// ===== 依赖导入 =====
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::stats::PetMode;

// ===== 资源分段枚举 =====
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Segment {
    A,
    C,
    Single,
}

impl Segment {
    fn stage_prefix(self) -> Option<&'static str> {
        match self {
            Segment::A => Some("A"),
            Segment::C => Some("C"),
            Segment::Single => None,
        }
    }
}

// ===== 通用资源路径与扫描工具 =====
pub(crate) fn body_asset_path(root: &str, relative: &str) -> PathBuf {
    PathBuf::from(root).join(relative)
}

pub(crate) fn pseudo_random_index(len: usize) -> usize {
    if len == 0 {
        return 0;
    }

    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as usize;
    seed % len
}

pub(crate) fn collect_png_files(asset_dir: &Path) -> Result<Vec<PathBuf>, String> {
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

pub(crate) fn collect_png_files_recursive_filtered(
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

// ===== 模式匹配辅助 =====
fn mode_keyword(mode: PetMode) -> &'static str {
    match mode {
        PetMode::Happy => "happy",
        PetMode::Nomal => "nomal",
        PetMode::PoorCondition => "poorcondition",
        PetMode::Ill => "ill",
    }
}

fn path_has_any_mode_keyword(path: &Path) -> bool {
    path.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .map(|name| {
                let lower = name.to_ascii_lowercase();
                lower.contains("happy")
                    || lower.contains("nomal")
                    || lower.contains("poorcondition")
                    || lower.contains("ill")
            })
            .unwrap_or(false)
    })
}

fn path_matches_mode(path: &Path, mode: PetMode) -> bool {
    path.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .map(|name| dir_name_matches_mode(name, mode))
            .unwrap_or(false)
    })
}

fn select_mode_or_agnostic_paths(paths: &[PathBuf], mode: PetMode) -> Vec<PathBuf> {
    paths
        .iter()
        .filter(|path| path_matches_mode(path, mode) || !path_has_any_mode_keyword(path))
        .cloned()
        .collect()
}

pub(crate) fn collect_png_files_recursive_for_mode(root: &Path, mode: PetMode) -> Vec<PathBuf> {
    let all_files = collect_png_files_recursive_filtered(root, &[]).unwrap_or_default();
    if all_files.is_empty() {
        return Vec::new();
    }

    let mut selected = select_mode_or_agnostic_paths(&all_files, mode);
    if selected.is_empty() && mode != PetMode::Nomal {
        selected = select_mode_or_agnostic_paths(&all_files, PetMode::Nomal);
    }
    if selected.is_empty() && mode != PetMode::Happy {
        selected = select_mode_or_agnostic_paths(&all_files, PetMode::Happy);
    }

    if selected.is_empty() {
        all_files
    } else {
        selected
    }
}

pub(crate) fn dir_name_matches_mode(dir_name: &str, mode: PetMode) -> bool {
    dir_name.to_ascii_lowercase().contains(mode_keyword(mode))
}

pub(crate) fn collect_dir_paths(root: &Path) -> Vec<PathBuf> {
    fs::read_dir(root)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(|entry| entry.ok().map(|item| item.path())))
        .filter(|path| path.is_dir())
        .collect()
}

pub(crate) fn collect_png_variant_dirs_recursive(root: &Path) -> Vec<PathBuf> {
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

pub(crate) fn collect_mode_variant_dirs(root: &Path, mode: PetMode) -> Vec<PathBuf> {
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

    if selected.is_empty() && mode != PetMode::Nomal {
        selected = variant_dirs
            .iter()
            .filter(|path| {
                path.file_name()
                    .and_then(|s| s.to_str())
                    .map(|name| dir_name_matches_mode(name, PetMode::Nomal))
                    .unwrap_or(false)
            })
            .cloned()
            .collect();
    }

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

// ===== 分段帧加载（A/C/Single + fallback） =====
fn path_matches_mode_for_segment(path: &Path, mode: PetMode) -> bool {
    path.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .map(|name| dir_name_matches_mode(name, mode))
            .unwrap_or(false)
    })
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

fn path_matches_mode_or_agnostic(path: &Path, mode: PetMode) -> bool {
    path_matches_mode_for_segment(path, mode) || !path_has_any_mode_keyword(path)
}

fn path_in_stage_branch(path: &Path, root: &Path, stage_prefix: &str) -> bool {
    let Ok(relative) = path.strip_prefix(root) else {
        return false;
    };

    relative.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .map(|name| component_matches_segment(name, stage_prefix))
            .unwrap_or(false)
    })
}

fn collect_segment_variants_for_mode(root: &Path, mode: PetMode, segment: Segment) -> Vec<Vec<PathBuf>> {
    let mut mode_dirs: Vec<PathBuf> = collect_png_variant_dirs_recursive(root)
        .into_iter()
        .filter(|path| path_matches_mode_or_agnostic(path, mode))
        .collect();

    if let Some(stage_prefix) = segment.stage_prefix() {
        mode_dirs.retain(|path| path_in_stage_branch(path, root, stage_prefix));
    }

    mode_dirs.sort();
    mode_dirs
        .into_iter()
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

fn load_frames_flat(root: &Path) -> Vec<PathBuf> {
    collect_png_files_recursive_filtered(root, &[]).unwrap_or_default()
}

pub(crate) fn load_frames_with_fallback(root: &Path, mode: PetMode, segment: Segment) -> Vec<PathBuf> {
    let mut variants = collect_segment_variants_for_mode(root, mode, segment);
    if variants.is_empty() && mode != PetMode::Nomal {
        variants = collect_segment_variants_for_mode(root, PetMode::Nomal, segment);
    }
    if variants.is_empty() && mode != PetMode::Happy {
        variants = collect_segment_variants_for_mode(root, PetMode::Happy, segment);
    }

    if variants.is_empty() {
        return match segment {
            Segment::Single => load_frames_flat(root),
            Segment::A | Segment::C => Vec::new(),
        };
    }

    variants.swap_remove(pseudo_random_index(variants.len()))
}
