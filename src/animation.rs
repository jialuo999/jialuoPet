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
    load_animation_path_config, AnimationPathConfig, CAROUSEL_INTERVAL_MS, STARTUP_EXCLUDED_DIRS,
};
use crate::input_region::setup_image_input_region;

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

static DRAG_RAISE_ANIMATION_PHASE: AtomicU8 = AtomicU8::new(DRAG_ANIM_IDLE);
static PINCH_ANIMATION_PHASE: AtomicU8 = AtomicU8::new(PINCH_ANIM_IDLE);
static SHUTDOWN_ANIMATION_PHASE: AtomicU8 = AtomicU8::new(SHUTDOWN_ANIM_IDLE);
static SHUTDOWN_ANIMATION_FINISHED: AtomicBool = AtomicBool::new(false);

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

fn choose_startup_animation_files(startup_root: &Path) -> Option<Vec<PathBuf>> {
    let startup_dirs: Vec<PathBuf> = fs::read_dir(startup_root)
        .ok()?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if !path.is_dir() {
                return None;
            }

            let dir_name = path.file_name()?.to_str()?;
            if STARTUP_EXCLUDED_DIRS
                .iter()
                .any(|excluded| dir_name.eq_ignore_ascii_case(excluded))
            {
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

fn collect_drag_raise_happy_files(raise_dynamic_root: &Path) -> Vec<PathBuf> {
    let happy_dir = raise_dynamic_root.join("Happy");
    if happy_dir.is_dir() {
        return collect_png_files_recursive_filtered(&happy_dir, STARTUP_EXCLUDED_DIRS)
            .unwrap_or_default();
    }

    Vec::new()
}

fn collect_drag_raise_start_files(raise_static_root: &Path) -> Vec<PathBuf> {
    collect_png_files(&raise_static_root.join("A_Happy")).unwrap_or_default()
}

fn collect_drag_raise_end_variants(raise_static_root: &Path) -> Vec<Vec<PathBuf>> {
    let end_dirs = ["C_Happy", "C_Happy_2"];
    end_dirs
        .iter()
        .filter_map(|name| {
            let files = collect_png_files(&raise_static_root.join(name)).ok()?;
            if files.is_empty() {
                None
            } else {
                Some(files)
            }
        })
        .collect()
}

fn collect_pinch_start_files(animation_config: &AnimationPathConfig) -> Vec<PathBuf> {
    collect_png_files(&body_asset_path(
        &animation_config.assets_body_root,
        "Pinch/Happy/A",
    ))
    .unwrap_or_default()
}

fn collect_pinch_loop_variants(animation_config: &AnimationPathConfig) -> Vec<Vec<PathBuf>> {
    ["Pinch/Happy/B", "Pinch/Happy/B_2"]
        .iter()
        .filter_map(|relative_dir| {
            let files = collect_png_files(&body_asset_path(
                &animation_config.assets_body_root,
                relative_dir,
            ))
            .ok()?;
            if files.is_empty() {
                None
            } else {
                Some(files)
            }
        })
        .collect()
}

fn collect_pinch_end_files(animation_config: &AnimationPathConfig) -> Vec<PathBuf> {
    collect_png_files(&body_asset_path(
        &animation_config.assets_body_root,
        "Pinch/Happy/C",
    ))
    .unwrap_or_default()
}

fn collect_shutdown_variants(animation_config: &AnimationPathConfig) -> Vec<Vec<PathBuf>> {
    animation_config
        .shutdown_variants
        .iter()
        .filter_map(|relative_dir| {
            let files = collect_png_files(&body_asset_path(
                &animation_config.assets_body_root,
                relative_dir,
            ))
            .ok()?;
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

struct CarouselState {
    startup_files: Vec<PathBuf>,
    startup_index: usize,
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
) -> Result<Image, String> {
    let animation_config = load_animation_path_config();
    let default_files = collect_default_happy_idle_files(&animation_config)?;
    if default_files.is_empty() {
        return Err("默认静息动画目录中没有找到 PNG 文件".to_string());
    }

    let startup_root = body_asset_path(
        &animation_config.assets_body_root,
        &animation_config.startup_root,
    );
    let startup_files = choose_startup_animation_files(&startup_root).unwrap_or_default();
    let playing_startup = !startup_files.is_empty();
    let drag_raise_dir = body_asset_path(
        &animation_config.assets_body_root,
        &animation_config.raise_dynamic_root,
    );
    let drag_raise_loop_files = collect_drag_raise_happy_files(&drag_raise_dir);
    let drag_raise_static_dir = body_asset_path(
        &animation_config.assets_body_root,
        &animation_config.raise_static_root,
    );
    let drag_raise_start_files = collect_drag_raise_start_files(&drag_raise_static_dir);
    let drag_raise_end_variants = collect_drag_raise_end_variants(&drag_raise_static_dir);
    let pinch_start_files = collect_pinch_start_files(&animation_config);
    let pinch_loop_variants = collect_pinch_loop_variants(&animation_config);
    let pinch_end_files = collect_pinch_end_files(&animation_config);
    let shutdown_variants = collect_shutdown_variants(&animation_config);

    let image = Image::new();
    image.set_pixel_size(256);

    let state = Rc::new(RefCell::new(CarouselState {
        startup_files,
        startup_index: 0,
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
        shutdown_variants,
        shutdown_files: Vec::new(),
        shutdown_index: 0,
        shutdown_hold_frame: None,
        playing_shutdown: false,
        drag_playback_mode: DragPlaybackMode::None,
        playing_startup,
    }));
    let state_clone = state.clone();
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
            let mut forced_frame: Option<PathBuf> = None;

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