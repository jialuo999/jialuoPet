use std::path::{Path, PathBuf};

use crate::stats_panel::PetMode;

use super::assets::{
    collect_png_files, collect_png_variant_dirs_recursive, path_contains_keyword,
    path_in_stage_branch, path_matches_mode, pseudo_random_index, touch_variant_key,
};

#[derive(Clone, Default)]
pub(super) struct TouchVariant {
    key: Option<String>,
    files: Vec<PathBuf>,
}

#[derive(Clone, Default)]
pub(super) struct TouchStageVariants {
    stage_a: Vec<TouchVariant>,
    stage_b: Vec<TouchVariant>,
    stage_c: Vec<TouchVariant>,
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

pub(super) fn collect_touch_variants(touch_root: &Path, mode: PetMode) -> TouchStageVariants {
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

pub(super) fn build_touch_sequence(variants: &TouchStageVariants) -> Vec<PathBuf> {
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
