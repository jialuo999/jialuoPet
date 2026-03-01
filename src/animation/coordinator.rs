use glib::timeout_add_local;
use gtk4::{ApplicationWindow, Image};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use crate::config::{load_animation_path_config, CAROUSEL_INTERVAL_MS};
use crate::input_region::setup_image_input_region;
use crate::stats_panel::{PetMode, PetStatsService};

use super::assets::body_asset_path;
use super::player::{
    AnimationPlayer, DefaultIdlePlayer, DragRaisePlayer, PinchPlayer, ShutdownPlayer,
    StartupPlayer, TouchPlayer,
};
use super::requests::{
    consume_requests, set_shutdown_animation_finished, AnimationRequests, DRAG_ANIM_END_REQUESTED,
    DRAG_ANIM_LOOP_REQUESTED, DRAG_ANIM_START_REQUESTED, PINCH_ANIM_END_REQUESTED,
    PINCH_ANIM_LOOP_REQUESTED, PINCH_ANIM_START_REQUESTED, SHUTDOWN_ANIM_REQUESTED,
    TOUCH_ANIM_BODY_REQUESTED, TOUCH_ANIM_HEAD_REQUESTED,
};

struct PlayerSet {
    current_mode: PetMode,
    shutdown: ShutdownPlayer,
    drag_raise: DragRaisePlayer,
    pinch: PinchPlayer,
    touch: TouchPlayer,
    startup: StartupPlayer,
    default_idle: DefaultIdlePlayer,
}

impl PlayerSet {
    fn reload_for_mode(&mut self, mode: PetMode) {
        self.current_mode = mode;
        self.default_idle.reload(mode);
        self.drag_raise.reload(mode);
        self.pinch.reload(mode);
        self.touch.reload(mode);
        self.shutdown.reload(mode);
    }

    fn initial_frame(&mut self) -> Option<PathBuf> {
        if self.startup.is_active() {
            self.startup.peek_first_frame()
        } else {
            self.default_idle.enter()
        }
    }
}

fn dispatch_requests(players: &mut PlayerSet, reqs: AnimationRequests) {
    if reqs.shutdown == SHUTDOWN_ANIM_REQUESTED {
        players.drag_raise.stop();
        players.pinch.stop();
        players.touch.stop();
        players.startup.stop();
        players.shutdown.start();
        return;
    }

    if players.shutdown.is_active() {
        return;
    }

    match reqs.drag {
        DRAG_ANIM_START_REQUESTED => {
            players.drag_raise.start(&mut players.pinch, &mut players.touch, &mut players.startup);
        }
        DRAG_ANIM_LOOP_REQUESTED => {
            players.drag_raise.continue_loop(&mut players.pinch, &mut players.touch, &mut players.startup);
        }
        DRAG_ANIM_END_REQUESTED => {
            players.drag_raise.end();
        }
        _ => {}
    }

    if !players.drag_raise.is_active() {
        match reqs.pinch {
            PINCH_ANIM_START_REQUESTED => {
                players.pinch.start(&mut players.touch, &mut players.startup);
            }
            PINCH_ANIM_LOOP_REQUESTED => {
                players.pinch.continue_loop(&mut players.touch, &mut players.startup);
            }
            PINCH_ANIM_END_REQUESTED => {
                players.pinch.end(&mut players.touch, &mut players.startup);
            }
            _ => {}
        }
    }

    if !players.drag_raise.is_active() && !players.pinch.is_active() {
        match reqs.touch {
            TOUCH_ANIM_HEAD_REQUESTED => players.touch.start_head(&mut players.startup),
            TOUCH_ANIM_BODY_REQUESTED => players.touch.start_body(&mut players.startup),
            _ => {}
        }
    }
}

fn advance_frame(players: &mut PlayerSet) -> PathBuf {
    if players.shutdown.is_active() {
        if let Some(frame) = players.shutdown.next_frame() {
            return frame;
        }
        players.shutdown.stop();
        return players.default_idle.enter().unwrap_or_default();
    }

    if players.drag_raise.is_active() {
        if let Some(frame) = players.drag_raise.next_frame() {
            return frame;
        }
        players.drag_raise.stop();
        return players.default_idle.enter().unwrap_or_default();
    }

    if players.pinch.is_active() {
        if let Some(frame) = players.pinch.next_frame() {
            return frame;
        }
        players.pinch.stop();
        return players.default_idle.enter().unwrap_or_default();
    }

    if players.touch.is_active() {
        if let Some(frame) = players.touch.next_frame() {
            return frame;
        }
        players.touch.stop();
        return players.default_idle.enter().unwrap_or_default();
    }

    if players.startup.is_active() {
        if let Some(frame) = players.startup.next_frame() {
            return frame;
        }
        players.startup.stop();
        return players.default_idle.enter().unwrap_or_default();
    }

    players
        .default_idle
        .next_frame()
        .or_else(|| players.default_idle.enter())
        .unwrap_or_default()
}

pub fn load_carousel_images(
    window: &ApplicationWindow,
    current_pixbuf: Rc<RefCell<Option<gdk_pixbuf::Pixbuf>>>,
    stats_service: PetStatsService,
) -> Result<Image, String> {
    let animation_config = load_animation_path_config();
    let current_mode = stats_service.cal_mode();

    let startup_root = body_asset_path(
        &animation_config.assets_body_root,
        &animation_config.startup_root,
    );
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

    let mut players = PlayerSet {
        current_mode,
        shutdown: ShutdownPlayer::new(shutdown_root, current_mode),
        drag_raise: DragRaisePlayer::new(drag_raise_dynamic_root, drag_raise_static_root, current_mode),
        pinch: PinchPlayer::new(pinch_root, current_mode),
        touch: TouchPlayer::new(touch_head_root, touch_body_root, current_mode),
        startup: StartupPlayer::new(startup_root, current_mode),
        default_idle: DefaultIdlePlayer::new(&animation_config, current_mode)?,
    };

    let image = Image::new();
    image.set_pixel_size(256);

    if let Some(first_frame) = players.initial_frame() {
        if let Ok(pixbuf) = gdk_pixbuf::Pixbuf::from_file(&first_frame) {
            image.set_from_pixbuf(Some(&pixbuf));
            *current_pixbuf.borrow_mut() = Some(pixbuf.clone());
            setup_image_input_region(window, &image, &pixbuf);
        }
    }

    let state = Rc::new(RefCell::new(players));
    let state_clone = state.clone();
    let stats_service_clone = stats_service.clone();
    let image_clone = image.clone();
    let window_clone = window.clone();

    timeout_add_local(Duration::from_millis(CAROUSEL_INTERVAL_MS), move || {
        let next_path = {
            let mut players = state_clone.borrow_mut();
            let reqs = consume_requests();

            if !players.startup.is_active() {
                let next_mode = stats_service_clone.cal_mode();
                if next_mode != players.current_mode {
                    players.reload_for_mode(next_mode);
                }
            }

            dispatch_requests(&mut players, reqs);
            advance_frame(&mut players)
        };

        if let Ok(pixbuf) = gdk_pixbuf::Pixbuf::from_file(&next_path) {
            image_clone.set_from_pixbuf(Some(&pixbuf));
            *current_pixbuf.borrow_mut() = Some(pixbuf.clone());
            setup_image_input_region(&window_clone, &image_clone, &pixbuf);
        }

        glib::ControlFlow::Continue
    });

    set_shutdown_animation_finished(false);

    Ok(image)
}
