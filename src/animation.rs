mod assets;
mod touch;
mod variants;

use glib::timeout_add_local;
use gtk4::{ApplicationWindow, Image};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::time::Duration;

use crate::config::{
    load_animation_path_config, CAROUSEL_INTERVAL_MS,
};
use crate::input_region::setup_image_input_region;
use crate::stats_panel::{PetMode, PetStatsService};
use assets::{body_asset_path, pseudo_random_index};
use touch::{build_touch_sequence, collect_touch_variants, TouchStageVariants};
use variants::{
    choose_startup_animation_files, collect_default_happy_idle_variants,
    collect_default_mode_idle_variants, collect_drag_raise_end_variants,
    collect_drag_raise_loop_files, collect_drag_raise_start_files, collect_pinch_end_files,
    collect_pinch_loop_variants, collect_pinch_start_files, collect_shutdown_variants,
    select_default_files_for_mode,
};

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

fn stop_touch_playback(state: &mut CarouselState) {
    state.touch_playback_mode = TouchPlaybackMode::None;
    state.touch_files.clear();
    state.touch_index = 0;
}

fn stop_startup_playback(state: &mut CarouselState) {
    state.playing_startup = false;
    state.startup_files.clear();
    state.startup_index = 0;
}

fn stop_pinch_playback(state: &mut CarouselState) {
    state.pinch_playback_mode = PinchPlaybackMode::None;
    state.pinch_loop_files.clear();
    state.pinch_start_index = 0;
    state.pinch_loop_index = 0;
    state.pinch_end_index = 0;
}

fn refresh_default_idle_selection(state: &mut CarouselState) {
    state.default_files = select_default_files_for_mode(
        state.current_mode,
        &state.default_happy_variants,
        &state.default_nomal_variants,
        &state.default_poor_condition_variants,
        &state.default_ill_variants,
    );
    state.default_index = 0;
}

fn enter_default_idle(state: &mut CarouselState) -> PathBuf {
    refresh_default_idle_selection(state);
    state.default_files[0].clone()
}

struct CarouselState {
    startup_files: Vec<PathBuf>,
    startup_index: usize,
    current_mode: PetMode,
    default_happy_variants: Vec<Vec<PathBuf>>,
    default_nomal_variants: Vec<Vec<PathBuf>>,
    default_poor_condition_variants: Vec<Vec<PathBuf>>,
    default_ill_variants: Vec<Vec<PathBuf>>,
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
    let default_happy_variants = collect_default_happy_idle_variants(&animation_config)?;
    if default_happy_variants.is_empty() {
        return Err("默认静息动画目录中没有找到 PNG 文件".to_string());
    }
    let default_nomal_variants = collect_default_mode_idle_variants(&animation_config, PetMode::Nomal);
    let default_poor_condition_variants =
        collect_default_mode_idle_variants(&animation_config, PetMode::PoorCondition);
    let default_ill_variants = collect_default_mode_idle_variants(&animation_config, PetMode::Ill);
    let current_mode = stats_service.cal_mode();
    let default_files = select_default_files_for_mode(
        current_mode,
        &default_happy_variants,
        &default_nomal_variants,
        &default_poor_condition_variants,
        &default_ill_variants,
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
        default_happy_variants,
        default_nomal_variants,
        default_poor_condition_variants,
        default_ill_variants,
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
                    refresh_default_idle_selection(&mut state_mut);
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
                        forced_frame = Some(enter_default_idle(&mut state_mut));
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
                    stop_startup_playback(&mut state_mut);
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
                    stop_startup_playback(&mut state_mut);
                    stop_touch_playback(&mut state_mut);
                    stop_pinch_playback(&mut state_mut);
                    state_mut.drag_playback_mode = DragPlaybackMode::None;
                    state_mut.drag_raise_start_index = 0;
                    state_mut.drag_raise_loop_index = 0;
                    state_mut.drag_raise_end_index = 0;
                    state_mut.drag_raise_end_files.clear();
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
                        stop_startup_playback(&mut state_mut);
                        stop_pinch_playback(&mut state_mut);
                        stop_touch_playback(&mut state_mut);
                        state_mut.drag_playback_mode = DragPlaybackMode::Start;
                        state_mut.drag_raise_start_index = 0;
                        forced_frame = Some(state_mut.drag_raise_start_files[0].clone());
                    } else if !state_mut.drag_raise_loop_files.is_empty() {
                        stop_startup_playback(&mut state_mut);
                        stop_pinch_playback(&mut state_mut);
                        stop_touch_playback(&mut state_mut);
                        state_mut.drag_playback_mode = DragPlaybackMode::Loop;
                        state_mut.drag_raise_loop_index = 0;
                        forced_frame = Some(state_mut.drag_raise_loop_files[0].clone());
                    }
                }
                DRAG_ANIM_LOOP_REQUESTED => {
                    if state_mut.drag_playback_mode != DragPlaybackMode::Start
                        && !state_mut.drag_raise_loop_files.is_empty()
                    {
                        stop_startup_playback(&mut state_mut);
                        stop_pinch_playback(&mut state_mut);
                        stop_touch_playback(&mut state_mut);
                        if state_mut.drag_playback_mode != DragPlaybackMode::Loop {
                            state_mut.drag_raise_loop_index = 0;
                            forced_frame = Some(state_mut.drag_raise_loop_files[0].clone());
                        }
                        state_mut.drag_playback_mode = DragPlaybackMode::Loop;
                    }
                }
                DRAG_ANIM_END_REQUESTED => {
                    if !state_mut.drag_raise_end_variants.is_empty() {
                        stop_startup_playback(&mut state_mut);
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
                            stop_startup_playback(&mut state_mut);
                            stop_touch_playback(&mut state_mut);
                            state_mut.pinch_playback_mode = PinchPlaybackMode::Start;
                            state_mut.pinch_start_index = 0;
                            forced_frame = Some(state_mut.pinch_start_files[0].clone());
                        } else if !state_mut.pinch_loop_variants.is_empty() {
                            stop_startup_playback(&mut state_mut);
                            stop_touch_playback(&mut state_mut);
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
                            stop_startup_playback(&mut state_mut);
                            stop_touch_playback(&mut state_mut);
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
                            stop_startup_playback(&mut state_mut);
                            stop_touch_playback(&mut state_mut);
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
                    enter_default_idle(&mut state_mut)
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
                        enter_default_idle(&mut state_mut)
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
                    if let Some(frame) = state_mut.pinch_loop_files.first().cloned() {
                        frame
                    } else {
                        enter_default_idle(&mut state_mut)
                    }
                } else {
                    state_mut.pinch_playback_mode = PinchPlaybackMode::None;
                    enter_default_idle(&mut state_mut)
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
                        enter_default_idle(&mut state_mut)
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
                        enter_default_idle(&mut state_mut)
                    }
                }
            } else if state_mut.playing_startup {
                let next_startup_index = state_mut.startup_index + 1;
                if next_startup_index < state_mut.startup_files.len() {
                    state_mut.startup_index = next_startup_index;
                    state_mut.startup_files[next_startup_index].clone()
                } else {
                    state_mut.playing_startup = false;
                    enter_default_idle(&mut state_mut)
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