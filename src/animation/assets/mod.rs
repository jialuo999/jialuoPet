use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::AnimationPathConfig;
use crate::stats_panel::PetMode;

#[derive(Clone, Default)]
pub(crate) struct TouchVariant {
    key: Option<String>,
    files: Vec<PathBuf>,
}

#[derive(Clone, Default)]
pub(crate) struct TouchStageVariants {
    stage_a: Vec<TouchVariant>,
    stage_b: Vec<TouchVariant>,
    stage_c: Vec<TouchVariant>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Segment {
    A,
    B,
    C,
    Single,
}

impl Segment {
    fn stage_prefix(self) -> Option<&'static str> {
        match self {
            Segment::A => Some("A"),
            Segment::B => Some("B"),
            Segment::C => Some("C"),
            Segment::Single => None,
        }
    }
}

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

fn mode_keyword(mode: PetMode) -> &'static str {
    match mode {
        PetMode::Happy => "happy",
        PetMode::Nomal => "nomal",
        PetMode::PoorCondition => "poorcondition",
        PetMode::Ill => "ill",
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

pub(crate) fn choose_startup_animation_files(
    startup_root: &Path,
    mode: PetMode,
) -> Option<Vec<PathBuf>> {
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
                dir_name_matches_mode(dir_name, PetMode::Happy)
                    || dir_name.eq_ignore_ascii_case("26new")
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

pub(crate) fn collect_default_happy_idle_variants(
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

pub(crate) fn collect_default_mode_idle_variants(
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

pub(crate) fn select_default_files_for_mode(
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
        .or_else(|| pick_random_variant(nomal_variants))
        .or_else(|| pick_random_variant(happy_variants))
        .unwrap_or_default()
}

fn collect_segment_variants_for_mode(root: &Path, mode: PetMode, segment: Segment) -> Vec<Vec<PathBuf>> {
    let mut mode_dirs: Vec<PathBuf> = collect_png_variant_dirs_recursive(root)
        .into_iter()
        .filter(|path| path_matches_mode(path, mode))
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
        return load_frames_flat(root);
    }

    variants.swap_remove(pseudo_random_index(variants.len()))
}

pub(crate) fn collect_drag_raise_loop_files(
    raise_dynamic_root: &Path,
    mode: PetMode,
) -> Vec<PathBuf> {
    load_frames_with_fallback(raise_dynamic_root, mode, Segment::Single)
}

pub(crate) fn collect_drag_raise_start_files(_raise_static_root: &Path, _mode: PetMode) -> Vec<PathBuf> {
    Vec::new()
}

pub(crate) fn collect_drag_raise_end_variants(
    raise_static_root: &Path,
    mode: PetMode,
) -> Vec<Vec<PathBuf>> {
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

    let selected_mode_dir = if mode_dirs.is_empty() && mode != PetMode::Nomal {
        collect_dir_paths(pinch_root)
            .into_iter()
            .find(|path| {
                path.file_name()
                    .and_then(|s| s.to_str())
                    .map(|name| dir_name_matches_mode(name, PetMode::Nomal))
                    .unwrap_or(false)
            })
    } else {
        mode_dirs.into_iter().next()
    }
    .or_else(|| {
        if mode != PetMode::Happy {
            collect_dir_paths(pinch_root)
                .into_iter()
                .find(|path| {
                    path.file_name()
                        .and_then(|s| s.to_str())
                        .map(|name| dir_name_matches_mode(name, PetMode::Happy))
                        .unwrap_or(false)
                })
        } else {
            None
        }
    });

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

pub(crate) fn collect_pinch_start_files(pinch_root: &Path, mode: PetMode) -> Vec<PathBuf> {
    let files = load_frames_with_fallback(pinch_root, mode, Segment::A);
    if files.is_empty() {
        let mut variants = collect_pinch_stage_variants(pinch_root, mode, "A");
        if variants.is_empty() {
            Vec::new()
        } else {
            variants.swap_remove(pseudo_random_index(variants.len()))
        }
    } else {
        files
    }
}

pub(crate) fn collect_pinch_loop_variants(pinch_root: &Path, mode: PetMode) -> Vec<Vec<PathBuf>> {
    collect_pinch_stage_variants(pinch_root, mode, "B")
}

pub(crate) fn collect_pinch_end_files(pinch_root: &Path, mode: PetMode) -> Vec<PathBuf> {
    let files = load_frames_with_fallback(pinch_root, mode, Segment::C);
    if files.is_empty() {
        let mut variants = collect_pinch_stage_variants(pinch_root, mode, "C");
        if variants.is_empty() {
            Vec::new()
        } else {
            variants.swap_remove(pseudo_random_index(variants.len()))
        }
    } else {
        files
    }
}

pub(crate) fn collect_shutdown_variants(shutdown_root: &Path, mode: PetMode) -> Vec<Vec<PathBuf>> {
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

fn path_matches_mode(path: &Path, mode: PetMode) -> bool {
    path.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .map(|name| dir_name_matches_mode(name, mode))
            .unwrap_or(false)
    })
}

fn path_contains_keyword(path: &Path, keyword: &str) -> bool {
    let keyword = keyword.to_ascii_lowercase();
    path.components().any(|component| {
        component
            .as_os_str()
            .to_str()
            .map(|name| name.to_ascii_lowercase().contains(&keyword))
            .unwrap_or(false)
    })
}

fn path_in_stage_branch(path: &Path, touch_root: &Path, stage_prefix: &str) -> bool {
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

fn touch_variant_key(path: &Path, touch_root: &Path, stage_prefix: &str) -> Option<String> {
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

fn collect_touch_stage_variants(
    touch_root: &Path,
    mode: PetMode,
    stage_prefix: &str,
) -> Vec<TouchVariant> {
    let candidate_dirs: Vec<PathBuf> = collect_png_variant_dirs_recursive(touch_root)
        .into_iter()
        .filter(|path| path_in_stage_branch(path, touch_root, stage_prefix))
        .collect();

    let pick_by_mode = |target_mode: PetMode| -> Vec<PathBuf> {
        candidate_dirs
            .iter()
            .filter(|path| path_matches_mode(path, target_mode))
            .cloned()
            .collect()
    };

    let mut mode_dirs: Vec<PathBuf> = pick_by_mode(mode);

    let is_touch_body = touch_root
        .file_name()
        .and_then(|s| s.to_str())
        .map(|name| name.eq_ignore_ascii_case("Touch_Body"))
        .unwrap_or(false);

    if is_touch_body && mode == PetMode::Happy {
        mode_dirs.retain(|path| !path_contains_keyword(path, "happy_turn"));
    }

    if mode_dirs.is_empty() && mode != PetMode::Nomal {
        mode_dirs = pick_by_mode(PetMode::Nomal);
    }

    if mode_dirs.is_empty() && mode != PetMode::Happy {
        mode_dirs = pick_by_mode(PetMode::Happy);
    }

    if is_touch_body {
        mode_dirs.retain(|path| !path_contains_keyword(path, "happy_turn"));
    }

    mode_dirs
        .into_iter()
        .filter_map(|path| {
            let files = collect_png_files(&path).ok()?;
            if files.is_empty() {
                None
            } else {
                Some(TouchVariant {
                    key: touch_variant_key(&path, touch_root, stage_prefix),
                    files,
                })
            }
        })
        .collect()
}

pub(crate) fn collect_touch_variants(touch_root: &Path, mode: PetMode) -> TouchStageVariants {
    TouchStageVariants {
        stage_a: collect_touch_stage_variants(touch_root, mode, "A"),
        stage_b: collect_touch_stage_variants(touch_root, mode, "B"),
        stage_c: collect_touch_stage_variants(touch_root, mode, "C"),
    }
}

fn extend_stage_sequence(
    output: &mut Vec<PathBuf>,
    variants: &[TouchVariant],
    selected_shared_key: Option<&str>,
) {
    if variants.is_empty() {
        return;
    }

    if let Some(key) = selected_shared_key {
        if let Some(variant) = variants
            .iter()
            .find(|variant| variant.key.as_deref() == Some(key))
        {
            output.extend(variant.files.iter().cloned());
            return;
        }
    }

    let index = pseudo_random_index(variants.len());
    output.extend(variants[index].files.iter().cloned());
}

pub(crate) fn build_touch_sequence(variants: &TouchStageVariants) -> Vec<PathBuf> {
    let mut shared_keys = Vec::new();
    for variant_a in &variants.stage_a {
        let Some(key) = variant_a.key.as_ref() else {
            continue;
        };
        let has_b = variants
            .stage_b
            .iter()
            .any(|variant| variant.key.as_deref() == Some(key.as_str()));
        let has_c = variants
            .stage_c
            .iter()
            .any(|variant| variant.key.as_deref() == Some(key.as_str()));
        if has_b && has_c {
            shared_keys.push(key.clone());
        }
    }

    let selected_shared_key = if shared_keys.is_empty() {
        None
    } else {
        Some(shared_keys[pseudo_random_index(shared_keys.len())].as_str())
    };

    let mut sequence = Vec::new();
    extend_stage_sequence(&mut sequence, &variants.stage_a, selected_shared_key);
    extend_stage_sequence(&mut sequence, &variants.stage_b, selected_shared_key);
    extend_stage_sequence(&mut sequence, &variants.stage_c, selected_shared_key);

    sequence
}
