use glib::timeout_add_local;
use gtk4::{ApplicationWindow, Image};
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::{
    load_animation_path_config, AnimationPathConfig, CAROUSEL_INTERVAL_MS,
};
use crate::input_region::setup_image_input_region;
use crate::stats_panel::{PetMode, PetStatsService};

const DRAG_ANIM_IDLE: u8 = 0;
const DRAG_ANIM_START_REQUESTED: u8 = 1;
const DRAG_ANIM_LOOP_REQUESTED: u8 = 2;
const DRAG_ANIM_END_REQUESTED: u8 = 3;
const PINCH_ANIM_IDLE: u8 = 0;
const PINCH_ANIM_START_REQUESTED: u8 = 1;
const PINCH_ANIM_LOOP_REQUESTED: u8 = 2;
const PINCH_ANIM_END_REQUESTED: u8 = 3;
const SHUTDOWN_ANIM_IDLE: u8 = 0;
const SHUTDOWN_ANIM_REQUESTED: u8 = 1;
const TOUCH_ANIM_IDLE: u8 = 0;
const TOUCH_ANIM_HEAD_REQUESTED: u8 = 1;
const TOUCH_ANIM_BODY_REQUESTED: u8 = 2;

static DRAG_RAISE_ANIMATION_PHASE: AtomicU8 = AtomicU8::new(DRAG_ANIM_IDLE);
static PINCH_ANIMATION_PHASE: AtomicU8 = AtomicU8::new(PINCH_ANIM_IDLE);
static SHUTDOWN_ANIMATION_PHASE: AtomicU8 = AtomicU8::new(SHUTDOWN_ANIM_IDLE);
static SHUTDOWN_ANIMATION_FINISHED: AtomicBool = AtomicBool::new(false);
static TOUCH_ANIMATION_PHASE: AtomicU8 = AtomicU8::new(TOUCH_ANIM_IDLE);

#[derive(Clone, Copy, PartialEq, Eq)]
enum DragPlaybackMode {
    None,
    Start,
    Loop,
    End,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PinchPlaybackMode {
    None,
    Start,
    Loop,
    End,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TouchPlaybackMode {
    None,
    Head,
    Body,
}

#[derive(Clone, Default)]
struct TouchStageVariants {
    stage_a: Vec<Vec<PathBuf>>,
    stage_b: Vec<Vec<PathBuf>>,
    stage_c: Vec<Vec<PathBuf>>,
}

pub fn request_drag_raise_animation_start() {
    DRAG_RAISE_ANIMATION_PHASE.store(DRAG_ANIM_START_REQUESTED, Ordering::Relaxed);
}

pub fn request_drag_raise_animation_loop() {
    DRAG_RAISE_ANIMATION_PHASE.store(DRAG_ANIM_LOOP_REQUESTED, Ordering::Relaxed);
}

pub fn request_drag_raise_animation_end() {
    DRAG_RAISE_ANIMATION_PHASE.store(DRAG_ANIM_END_REQUESTED, Ordering::Relaxed);
}

pub fn request_pinch_animation_start() {
    PINCH_ANIMATION_PHASE.store(PINCH_ANIM_START_REQUESTED, Ordering::Relaxed);
}

pub fn request_pinch_animation_end() {
    PINCH_ANIMATION_PHASE.store(PINCH_ANIM_END_REQUESTED, Ordering::Relaxed);
}

pub fn request_shutdown_animation() {
    SHUTDOWN_ANIMATION_FINISHED.store(false, Ordering::Relaxed);
    SHUTDOWN_ANIMATION_PHASE.store(SHUTDOWN_ANIM_REQUESTED, Ordering::Relaxed);
}

pub fn request_touch_head_animation() {
    TOUCH_ANIMATION_PHASE.store(TOUCH_ANIM_HEAD_REQUESTED, Ordering::Relaxed);
}

pub fn request_touch_body_animation() {
    TOUCH_ANIMATION_PHASE.store(TOUCH_ANIM_BODY_REQUESTED, Ordering::Relaxed);
}

pub fn is_shutdown_animation_finished() -> bool {
    SHUTDOWN_ANIMATION_FINISHED.load(Ordering::Relaxed)
}

fn body_asset_path(root: &str, relative: &str) -> PathBuf {
    PathBuf::from(root).join(relative)
}

fn pseudo_random_index(len: usize) -> usize {
    if len == 0 {
        return 0;
    }

    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as usize;
    seed % len
}

fn collect_png_files(asset_dir: &Path) -> Result<Vec<PathBuf>, String> {
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

fn collect_png_files_recursive_filtered(
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

fn dir_name_matches_mode(dir_name: &str, mode: PetMode) -> bool {
    dir_name.to_ascii_lowercase().contains(mode_keyword(mode))
}

fn collect_dir_paths(root: &Path) -> Vec<PathBuf> {
    fs::read_dir(root)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(|entry| entry.ok().map(|item| item.path())))
        .filter(|path| path.is_dir())
        .collect()
}

fn collect_png_variant_dirs_recursive(root: &Path) -> Vec<PathBuf> {
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

fn collect_mode_variant_dirs(root: &Path, mode: PetMode) -> Vec<PathBuf> {
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

fn choose_startup_animation_files(startup_root: &Path, mode: PetMode) -> Option<Vec<PathBuf>> {
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

fn collect_drag_raise_loop_files(raise_dynamic_root: &Path, mode: PetMode) -> Vec<PathBuf> {
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

fn collect_drag_raise_start_files(_raise_static_root: &Path, _mode: PetMode) -> Vec<PathBuf> {
    Vec::new()
}

fn collect_drag_raise_end_variants(raise_static_root: &Path, mode: PetMode) -> Vec<Vec<PathBuf>> {
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

fn collect_pinch_start_files(pinch_root: &Path, mode: PetMode) -> Vec<PathBuf> {
    let mut variants = collect_pinch_stage_variants(pinch_root, mode, "A");
    if variants.is_empty() {
        Vec::new()
    } else {
        variants.swap_remove(pseudo_random_index(variants.len()))
    }
}

fn collect_pinch_loop_variants(pinch_root: &Path, mode: PetMode) -> Vec<Vec<PathBuf>> {
    collect_pinch_stage_variants(pinch_root, mode, "B")
}

fn collect_pinch_end_files(pinch_root: &Path, mode: PetMode) -> Vec<PathBuf> {
    let mut variants = collect_pinch_stage_variants(pinch_root, mode, "C");
    if variants.is_empty() {
        Vec::new()
    } else {
        variants.swap_remove(pseudo_random_index(variants.len()))
    }
}

fn collect_touch_stage_variants(
    touch_root: &Path,
    mode: PetMode,
    stage_prefix: &str,
) -> Vec<Vec<PathBuf>> {
    let candidate_dirs: Vec<PathBuf> = collect_png_variant_dirs_recursive(touch_root)
        .into_iter()
        .filter(|path| path_in_stage_branch(path, touch_root, stage_prefix))
        .collect();

    let mut mode_dirs: Vec<PathBuf> = candidate_dirs
        .iter()
        .filter(|path| path_matches_mode(path, mode))
        .cloned()
        .collect();

    let is_touch_body = touch_root
        .file_name()
        .and_then(|s| s.to_str())
        .map(|name| name.eq_ignore_ascii_case("Touch_Body"))
        .unwrap_or(false);
    if is_touch_body && mode == PetMode::Happy {
        mode_dirs.retain(|path| !path_contains_keyword(path, "happy_turn"));
    }

    if mode_dirs.is_empty() && mode != PetMode::Happy {
        mode_dirs = candidate_dirs
            .into_iter()
            .filter(|path| path_matches_mode(path, PetMode::Happy))
            .collect();
        if is_touch_body {
            mode_dirs.retain(|path| !path_contains_keyword(path, "happy_turn"));
        }
    }

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

fn collect_touch_variants(touch_root: &Path, mode: PetMode) -> TouchStageVariants {
    TouchStageVariants {
        stage_a: collect_touch_stage_variants(touch_root, mode, "A"),
        stage_b: collect_touch_stage_variants(touch_root, mode, "B"),
        stage_c: collect_touch_stage_variants(touch_root, mode, "C"),
    }
}

fn build_touch_sequence(variants: &TouchStageVariants) -> Vec<PathBuf> {
    let mut sequence = Vec::new();

    if !variants.stage_a.is_empty() {
        let index = pseudo_random_index(variants.stage_a.len());
        sequence.extend(variants.stage_a[index].iter().cloned());
    }
    if !variants.stage_b.is_empty() {
        let index = pseudo_random_index(variants.stage_b.len());
        sequence.extend(variants.stage_b[index].iter().cloned());
    }
    if !variants.stage_c.is_empty() {
        let index = pseudo_random_index(variants.stage_c.len());
        sequence.extend(variants.stage_c[index].iter().cloned());
    }

    sequence
}

fn stop_touch_playback(state: &mut CarouselState) {
    state.touch_playback_mode = TouchPlaybackMode::None;
    state.touch_files.clear();
    state.touch_index = 0;
}

fn collect_shutdown_variants(shutdown_root: &Path, mode: PetMode) -> Vec<Vec<PathBuf>> {
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

fn collect_default_happy_idle_files(
    animation_config: &AnimationPathConfig,
) -> Result<Vec<PathBuf>, String> {
    let mut all_files = Vec::new();
    for variant in &animation_config.default_happy_idle_variants {
        let dir = body_asset_path(&animation_config.assets_body_root, variant);
        let mut files = collect_png_files(&dir)?;
        all_files.append(&mut files);
    }
    Ok(all_files)
}

fn collect_default_mode_idle_files(
    animation_config: &AnimationPathConfig,
    mode: PetMode,
) -> Vec<PathBuf> {
    match mode {
        PetMode::Happy => collect_default_happy_idle_files(animation_config).unwrap_or_default(),
        PetMode::Nomal => collect_png_files_recursive_filtered(
            &body_asset_path(
                &animation_config.assets_body_root,
                &animation_config.default_nomal_idle_root,
            ),
            &[],
        )
        .unwrap_or_default(),
        PetMode::PoorCondition => collect_png_files_recursive_filtered(
            &body_asset_path(
                &animation_config.assets_body_root,
                &animation_config.default_poor_condition_idle_root,
            ),
            &[],
        )
        .unwrap_or_default(),
        PetMode::Ill => collect_png_files_recursive_filtered(
            &body_asset_path(
                &animation_config.assets_body_root,
                &animation_config.default_ill_idle_root,
            ),
            &[],
        )
        .unwrap_or_default(),
    }
}

fn select_default_files_for_mode(
    mode: PetMode,
    happy_files: &[PathBuf],
    nomal_files: &[PathBuf],
    poor_condition_files: &[PathBuf],
    ill_files: &[PathBuf],
) -> Vec<PathBuf> {
    let selected = match mode {
        PetMode::Happy => happy_files,
        PetMode::Nomal => nomal_files,
        PetMode::PoorCondition => poor_condition_files,
        PetMode::Ill => ill_files,
    };

    if selected.is_empty() {
        happy_files.to_vec()
    } else {
        selected.to_vec()
    }
}

struct CarouselState {
    startup_files: Vec<PathBuf>,
    startup_index: usize,
    current_mode: PetMode,
    default_happy_files: Vec<PathBuf>,
    default_nomal_files: Vec<PathBuf>,
    default_poor_condition_files: Vec<PathBuf>,
    default_ill_files: Vec<PathBuf>,
    default_files: Vec<PathBuf>,
    default_index: usize,
    drag_raise_start_files: Vec<PathBuf>,
    drag_raise_start_index: usize,
    drag_raise_loop_files: Vec<PathBuf>,
    drag_raise_loop_index: usize,
    drag_raise_end_variants: Vec<Vec<PathBuf>>,
    drag_raise_end_files: Vec<PathBuf>,
    drag_raise_end_index: usize,
    pinch_start_files: Vec<PathBuf>,
    pinch_start_index: usize,
    pinch_loop_variants: Vec<Vec<PathBuf>>,
    pinch_loop_files: Vec<PathBuf>,
    pinch_loop_index: usize,
    pinch_end_files: Vec<PathBuf>,
    pinch_end_index: usize,
    pinch_playback_mode: PinchPlaybackMode,
    touch_head_variants: TouchStageVariants,
    touch_body_variants: TouchStageVariants,
    touch_files: Vec<PathBuf>,
    touch_index: usize,
    touch_playback_mode: TouchPlaybackMode,
    shutdown_variants: Vec<Vec<PathBuf>>,
    shutdown_files: Vec<PathBuf>,
    shutdown_index: usize,
    shutdown_hold_frame: Option<PathBuf>,
    playing_shutdown: bool,
    drag_playback_mode: DragPlaybackMode,
    playing_startup: bool,
}

pub fn load_carousel_images(
    window: &ApplicationWindow,
    current_pixbuf: Rc<RefCell<Option<gdk_pixbuf::Pixbuf>>>,
    stats_service: PetStatsService,
) -> Result<Image, String> {
    let animation_config = load_animation_path_config();
    let default_happy_files = collect_default_happy_idle_files(&animation_config)?;
    if default_happy_files.is_empty() {
        return Err("默认静息动画目录中没有找到 PNG 文件".to_string());
    }
    let default_nomal_files = collect_default_mode_idle_files(&animation_config, PetMode::Nomal);
    let default_poor_condition_files =
        collect_default_mode_idle_files(&animation_config, PetMode::PoorCondition);
    let default_ill_files = collect_default_mode_idle_files(&animation_config, PetMode::Ill);
    let current_mode = stats_service.cal_mode();
    let default_files = select_default_files_for_mode(
        current_mode,
        &default_happy_files,
        &default_nomal_files,
        &default_poor_condition_files,
        &default_ill_files,
    );

    let startup_root = body_asset_path(
        &animation_config.assets_body_root,
        &animation_config.startup_root,
    );
    let startup_files = choose_startup_animation_files(&startup_root, current_mode).unwrap_or_default();
    let playing_startup = !startup_files.is_empty();
    let drag_raise_dynamic_root = body_asset_path(
        &animation_config.assets_body_root,
        &animation_config.raise_dynamic_root,
    );
    let drag_raise_static_root = body_asset_path(
        &animation_config.assets_body_root,
        &animation_config.raise_static_root,
    );
    let pinch_root = body_asset_path(&animation_config.assets_body_root, &animation_config.pinch_root);
    let shutdown_root = body_asset_path(
        &animation_config.assets_body_root,
        &animation_config.shutdown_root,
    );
    let touch_head_root = body_asset_path(
        &animation_config.assets_body_root,
        &animation_config.touch_head_root,
    );
    let touch_body_root = body_asset_path(
        &animation_config.assets_body_root,
        &animation_config.touch_body_root,
    );

    let drag_raise_start_files = collect_drag_raise_start_files(&drag_raise_static_root, current_mode);
    let drag_raise_loop_files = collect_drag_raise_loop_files(&drag_raise_dynamic_root, current_mode);
    let drag_raise_end_variants = collect_drag_raise_end_variants(&drag_raise_static_root, current_mode);
    let pinch_start_files = collect_pinch_start_files(&pinch_root, current_mode);
    let pinch_loop_variants = collect_pinch_loop_variants(&pinch_root, current_mode);
    let pinch_end_files = collect_pinch_end_files(&pinch_root, current_mode);
    let touch_head_variants = collect_touch_variants(&touch_head_root, current_mode);
    let touch_body_variants = collect_touch_variants(&touch_body_root, current_mode);
    let shutdown_variants = collect_shutdown_variants(&shutdown_root, current_mode);

    let image = Image::new();
    image.set_pixel_size(256);

    let state = Rc::new(RefCell::new(CarouselState {
        startup_files,
        startup_index: 0,
        current_mode,
        default_happy_files,
        default_nomal_files,
        default_poor_condition_files,
        default_ill_files,
        default_files,
        default_index: 0,
        drag_raise_start_files,
        drag_raise_start_index: 0,
        drag_raise_loop_files,
        drag_raise_loop_index: 0,
        drag_raise_end_variants,
        drag_raise_end_files: Vec::new(),
        drag_raise_end_index: 0,
        pinch_start_files,
        pinch_start_index: 0,
        pinch_loop_variants,
        pinch_loop_files: Vec::new(),
        pinch_loop_index: 0,
        pinch_end_files,
        pinch_end_index: 0,
        pinch_playback_mode: PinchPlaybackMode::None,
        touch_head_variants,
        touch_body_variants,
        touch_files: Vec::new(),
        touch_index: 0,
        touch_playback_mode: TouchPlaybackMode::None,
        shutdown_variants,
        shutdown_files: Vec::new(),
        shutdown_index: 0,
        shutdown_hold_frame: None,
        playing_shutdown: false,
        drag_playback_mode: DragPlaybackMode::None,
        playing_startup,
    }));
    let state_clone = state.clone();
    let stats_service_clone = stats_service.clone();
    let drag_raise_dynamic_root_clone = drag_raise_dynamic_root.clone();
    let drag_raise_static_root_clone = drag_raise_static_root.clone();
    let pinch_root_clone = pinch_root.clone();
    let shutdown_root_clone = shutdown_root.clone();
    let touch_head_root_clone = touch_head_root.clone();
    let touch_body_root_clone = touch_body_root.clone();
    let image_clone = image.clone();
    let window_clone = window.clone();

    let first_frame = {
        let state_ref = state.borrow();
        if state_ref.playing_startup {
            state_ref.startup_files[0].clone()
        } else {
            state_ref.default_files[0].clone()
        }
    };

    if let Ok(pixbuf) = gdk_pixbuf::Pixbuf::from_file(&first_frame) {
        image_clone.set_from_pixbuf(Some(&pixbuf));
        *current_pixbuf.borrow_mut() = Some(pixbuf.clone());
        setup_image_input_region(&window_clone, &image_clone, &pixbuf);
    }

    timeout_add_local(Duration::from_millis(CAROUSEL_INTERVAL_MS), move || {
        let next_path = {
            let mut state_mut = state_clone.borrow_mut();
            let shutdown_request =
                SHUTDOWN_ANIMATION_PHASE.swap(SHUTDOWN_ANIM_IDLE, Ordering::Relaxed);
            let drag_request = DRAG_RAISE_ANIMATION_PHASE.swap(DRAG_ANIM_IDLE, Ordering::Relaxed);
            let pinch_request = PINCH_ANIMATION_PHASE.swap(PINCH_ANIM_IDLE, Ordering::Relaxed);
            let touch_request = TOUCH_ANIMATION_PHASE.swap(TOUCH_ANIM_IDLE, Ordering::Relaxed);
            let mut forced_frame: Option<PathBuf> = None;

            if !state_mut.playing_startup {
                let next_mode = stats_service_clone.cal_mode();
                if next_mode != state_mut.current_mode {
                    state_mut.current_mode = next_mode;
                    state_mut.default_files = select_default_files_for_mode(
                        next_mode,
                        &state_mut.default_happy_files,
                        &state_mut.default_nomal_files,
                        &state_mut.default_poor_condition_files,
                        &state_mut.default_ill_files,
                    );
                    state_mut.default_index = 0;
                    state_mut.drag_raise_start_files =
                        collect_drag_raise_start_files(&drag_raise_static_root_clone, next_mode);
                    state_mut.drag_raise_start_index = 0;
                    state_mut.drag_raise_loop_files =
                        collect_drag_raise_loop_files(&drag_raise_dynamic_root_clone, next_mode);
                    state_mut.drag_raise_loop_index = 0;
                    state_mut.drag_raise_end_variants =
                        collect_drag_raise_end_variants(&drag_raise_static_root_clone, next_mode);
                    state_mut.drag_raise_end_files.clear();
                    state_mut.drag_raise_end_index = 0;
                    state_mut.pinch_start_files = collect_pinch_start_files(&pinch_root_clone, next_mode);
                    state_mut.pinch_start_index = 0;
                    state_mut.pinch_loop_variants =
                        collect_pinch_loop_variants(&pinch_root_clone, next_mode);
                    state_mut.pinch_loop_files.clear();
                    state_mut.pinch_loop_index = 0;
                    state_mut.pinch_end_files = collect_pinch_end_files(&pinch_root_clone, next_mode);
                    state_mut.pinch_end_index = 0;
                    state_mut.touch_head_variants =
                        collect_touch_variants(&touch_head_root_clone, next_mode);
                    state_mut.touch_body_variants =
                        collect_touch_variants(&touch_body_root_clone, next_mode);
                    state_mut.touch_files.clear();
                    state_mut.touch_index = 0;
                    state_mut.touch_playback_mode = TouchPlaybackMode::None;
                    state_mut.shutdown_variants = collect_shutdown_variants(&shutdown_root_clone, next_mode);
                    if !state_mut.playing_shutdown
                        && state_mut.drag_playback_mode == DragPlaybackMode::None
                        && state_mut.pinch_playback_mode == PinchPlaybackMode::None
                    {
                        forced_frame = state_mut.default_files.first().cloned();
                    }
                }
            }

            if !state_mut.playing_shutdown
                && state_mut.drag_playback_mode == DragPlaybackMode::None
                && state_mut.pinch_playback_mode == PinchPlaybackMode::None
                && state_mut.touch_playback_mode == TouchPlaybackMode::None
            {
                let (playback_mode, sequence) = match touch_request {
                    TOUCH_ANIM_HEAD_REQUESTED => (
                        TouchPlaybackMode::Head,
                        build_touch_sequence(&state_mut.touch_head_variants),
                    ),
                    TOUCH_ANIM_BODY_REQUESTED => (
                        TouchPlaybackMode::Body,
                        build_touch_sequence(&state_mut.touch_body_variants),
                    ),
                    _ => (TouchPlaybackMode::None, Vec::new()),
                };

                if playback_mode != TouchPlaybackMode::None && !sequence.is_empty() {
                    state_mut.touch_playback_mode = playback_mode;
                    state_mut.touch_files = sequence;
                    state_mut.touch_index = 0;
                    forced_frame = state_mut.touch_files.first().cloned();
                }
            }

            if shutdown_request == SHUTDOWN_ANIM_REQUESTED {
                if state_mut.shutdown_variants.is_empty() {
                    SHUTDOWN_ANIMATION_FINISHED.store(true, Ordering::Relaxed);
                } else {
                    let variant_index = pseudo_random_index(state_mut.shutdown_variants.len());
                    state_mut.shutdown_files = state_mut.shutdown_variants[variant_index].clone();
                    state_mut.shutdown_index = 0;
                    state_mut.playing_shutdown = true;
                    state_mut.shutdown_hold_frame = state_mut.shutdown_files.first().cloned();
                    forced_frame = state_mut.shutdown_hold_frame.clone();
                }
            }

            if !state_mut.playing_shutdown {
                match drag_request {
                DRAG_ANIM_START_REQUESTED => {
                    stop_touch_playback(&mut state_mut);
                    if !state_mut.drag_raise_start_files.is_empty() {
                        state_mut.drag_playback_mode = DragPlaybackMode::Start;
                        state_mut.drag_raise_start_index = 0;
                        forced_frame = Some(state_mut.drag_raise_start_files[0].clone());
                    } else if !state_mut.drag_raise_loop_files.is_empty() {
                        state_mut.drag_playback_mode = DragPlaybackMode::Loop;
                        state_mut.drag_raise_loop_index = 0;
                        forced_frame = Some(state_mut.drag_raise_loop_files[0].clone());
                    }
                }
                DRAG_ANIM_LOOP_REQUESTED => {
                    stop_touch_playback(&mut state_mut);
                    if state_mut.drag_playback_mode != DragPlaybackMode::Start
                        && !state_mut.drag_raise_loop_files.is_empty()
                    {
                        if state_mut.drag_playback_mode != DragPlaybackMode::Loop {
                            state_mut.drag_raise_loop_index = 0;
                            forced_frame = Some(state_mut.drag_raise_loop_files[0].clone());
                        }
                        state_mut.drag_playback_mode = DragPlaybackMode::Loop;
                    }
                }
                DRAG_ANIM_END_REQUESTED => {
                    if !state_mut.drag_raise_end_variants.is_empty() {
                        let variant_index = pseudo_random_index(state_mut.drag_raise_end_variants.len());
                        state_mut.drag_raise_end_files =
                            state_mut.drag_raise_end_variants[variant_index].clone();
                        state_mut.drag_raise_end_index = 0;
                        state_mut.drag_playback_mode = DragPlaybackMode::End;
                        forced_frame = state_mut.drag_raise_end_files.first().cloned();
                    } else {
                        state_mut.drag_playback_mode = DragPlaybackMode::None;
                    }
                }
                _ => {}
            }

            let drag_is_playing = state_mut.drag_playback_mode != DragPlaybackMode::None;
            if !drag_is_playing {
                match pinch_request {
                    PINCH_ANIM_START_REQUESTED => {
                        if !state_mut.pinch_start_files.is_empty() {
                            state_mut.pinch_playback_mode = PinchPlaybackMode::Start;
                            state_mut.pinch_start_index = 0;
                            forced_frame = Some(state_mut.pinch_start_files[0].clone());
                        } else if !state_mut.pinch_loop_variants.is_empty() {
                            let variant_index = pseudo_random_index(state_mut.pinch_loop_variants.len());
                            state_mut.pinch_loop_files = state_mut.pinch_loop_variants[variant_index].clone();
                            state_mut.pinch_loop_index = 0;
                            state_mut.pinch_playback_mode = PinchPlaybackMode::Loop;
                            forced_frame = state_mut.pinch_loop_files.first().cloned();
                        }
                    }
                    PINCH_ANIM_LOOP_REQUESTED => {
                        if state_mut.pinch_playback_mode != PinchPlaybackMode::Start
                            && !state_mut.pinch_loop_variants.is_empty()
                        {
                            if state_mut.pinch_playback_mode != PinchPlaybackMode::Loop
                                || state_mut.pinch_loop_files.is_empty()
                            {
                                let variant_index =
                                    pseudo_random_index(state_mut.pinch_loop_variants.len());
                                state_mut.pinch_loop_files =
                                    state_mut.pinch_loop_variants[variant_index].clone();
                                state_mut.pinch_loop_index = 0;
                                forced_frame = state_mut.pinch_loop_files.first().cloned();
                            }
                            state_mut.pinch_playback_mode = PinchPlaybackMode::Loop;
                        }
                    }
                    PINCH_ANIM_END_REQUESTED => {
                        if !state_mut.pinch_end_files.is_empty() {
                            state_mut.pinch_playback_mode = PinchPlaybackMode::End;
                            state_mut.pinch_end_index = 0;
                            forced_frame = Some(state_mut.pinch_end_files[0].clone());
                        } else {
                            state_mut.pinch_playback_mode = PinchPlaybackMode::None;
                        }
                    }
                    _ => {}
                }
            }
            }

            if let Some(frame) = forced_frame {
                frame
            } else if state_mut.playing_shutdown {
                let next_index = state_mut.shutdown_index + 1;
                if next_index < state_mut.shutdown_files.len() {
                    state_mut.shutdown_index = next_index;
                    let frame = state_mut.shutdown_files[next_index].clone();
                    state_mut.shutdown_hold_frame = Some(frame.clone());
                    frame
                } else {
                    state_mut.playing_shutdown = false;
                    SHUTDOWN_ANIMATION_FINISHED.store(true, Ordering::Relaxed);
                    state_mut
                        .shutdown_hold_frame
                        .clone()
                        .unwrap_or_else(|| state_mut.default_files[state_mut.default_index].clone())
                }
            } else if let Some(frame) = state_mut.shutdown_hold_frame.clone() {
                frame
            } else if state_mut.drag_playback_mode == DragPlaybackMode::Start {
                let next_index = state_mut.drag_raise_start_index + 1;
                if next_index < state_mut.drag_raise_start_files.len() {
                    state_mut.drag_raise_start_index = next_index;
                    state_mut.drag_raise_start_files[next_index].clone()
                } else if !state_mut.drag_raise_loop_files.is_empty() {
                    state_mut.drag_playback_mode = DragPlaybackMode::Loop;
                    state_mut.drag_raise_loop_index = 0;
                    state_mut.drag_raise_loop_files[0].clone()
                } else {
                    state_mut.drag_playback_mode = DragPlaybackMode::None;
                    state_mut.default_files[state_mut.default_index].clone()
                }
            } else if state_mut.drag_playback_mode == DragPlaybackMode::Loop
                && !state_mut.drag_raise_loop_files.is_empty()
            {
                let next_index = (state_mut.drag_raise_loop_index + 1) % state_mut.drag_raise_loop_files.len();
                state_mut.drag_raise_loop_index = next_index;
                state_mut.drag_raise_loop_files[next_index].clone()
            } else if state_mut.drag_playback_mode == DragPlaybackMode::End {
                let next_index = state_mut.drag_raise_end_index + 1;
                if next_index < state_mut.drag_raise_end_files.len() {
                    state_mut.drag_raise_end_index = next_index;
                    state_mut.drag_raise_end_files[next_index].clone()
                } else {
                    state_mut.drag_playback_mode = DragPlaybackMode::None;
                    if state_mut.playing_startup {
                        state_mut.startup_files[state_mut.startup_index].clone()
                    } else {
                        state_mut.default_files[state_mut.default_index].clone()
                    }
                }
            } else if state_mut.pinch_playback_mode == PinchPlaybackMode::Start {
                let next_index = state_mut.pinch_start_index + 1;
                if next_index < state_mut.pinch_start_files.len() {
                    state_mut.pinch_start_index = next_index;
                    state_mut.pinch_start_files[next_index].clone()
                } else if !state_mut.pinch_loop_variants.is_empty() {
                    let variant_index = pseudo_random_index(state_mut.pinch_loop_variants.len());
                    state_mut.pinch_loop_files = state_mut.pinch_loop_variants[variant_index].clone();
                    state_mut.pinch_loop_index = 0;
                    state_mut.pinch_playback_mode = PinchPlaybackMode::Loop;
                    state_mut
                        .pinch_loop_files
                        .first()
                        .cloned()
                        .unwrap_or_else(|| state_mut.default_files[state_mut.default_index].clone())
                } else {
                    state_mut.pinch_playback_mode = PinchPlaybackMode::None;
                    state_mut.default_files[state_mut.default_index].clone()
                }
            } else if state_mut.pinch_playback_mode == PinchPlaybackMode::Loop
                && !state_mut.pinch_loop_files.is_empty()
            {
                let next_index = state_mut.pinch_loop_index + 1;
                if next_index < state_mut.pinch_loop_files.len() {
                    state_mut.pinch_loop_index = next_index;
                    state_mut.pinch_loop_files[next_index].clone()
                } else {
                    let variant_index = pseudo_random_index(state_mut.pinch_loop_variants.len());
                    state_mut.pinch_loop_files = state_mut.pinch_loop_variants[variant_index].clone();
                    state_mut.pinch_loop_index = 0;
                    state_mut
                        .pinch_loop_files
                        .first()
                        .cloned()
                        .unwrap_or_else(|| state_mut.default_files[state_mut.default_index].clone())
                }
            } else if state_mut.pinch_playback_mode == PinchPlaybackMode::End {
                let next_index = state_mut.pinch_end_index + 1;
                if next_index < state_mut.pinch_end_files.len() {
                    state_mut.pinch_end_index = next_index;
                    state_mut.pinch_end_files[next_index].clone()
                } else {
                    state_mut.pinch_playback_mode = PinchPlaybackMode::None;
                    if state_mut.playing_startup {
                        state_mut.startup_files[state_mut.startup_index].clone()
                    } else {
                        state_mut.default_files[state_mut.default_index].clone()
                    }
                }
            } else if state_mut.touch_playback_mode != TouchPlaybackMode::None {
                let next_index = state_mut.touch_index + 1;
                if next_index < state_mut.touch_files.len() {
                    state_mut.touch_index = next_index;
                    state_mut.touch_files[next_index].clone()
                } else {
                    state_mut.touch_playback_mode = TouchPlaybackMode::None;
                    state_mut.touch_files.clear();
                    state_mut.touch_index = 0;
                    if state_mut.playing_startup {
                        state_mut.startup_files[state_mut.startup_index].clone()
                    } else {
                        state_mut.default_files[state_mut.default_index].clone()
                    }
                }
            } else if state_mut.playing_startup {
                let next_startup_index = state_mut.startup_index + 1;
                if next_startup_index < state_mut.startup_files.len() {
                    state_mut.startup_index = next_startup_index;
                    state_mut.startup_files[next_startup_index].clone()
                } else {
                    state_mut.playing_startup = false;
                    state_mut.default_index = 0;
                    state_mut.default_files[0].clone()
                }
            } else {
                let next_index = (state_mut.default_index + 1) % state_mut.default_files.len();
                state_mut.default_index = next_index;
                state_mut.default_files[next_index].clone()
            }
        };

        if let Ok(pixbuf) = gdk_pixbuf::Pixbuf::from_file(&next_path) {
            image_clone.set_from_pixbuf(Some(&pixbuf));
            *current_pixbuf.borrow_mut() = Some(pixbuf.clone());
            setup_image_input_region(&window_clone, &image_clone, &pixbuf);
        }

        glib::ControlFlow::Continue
    });

    Ok(image)
}