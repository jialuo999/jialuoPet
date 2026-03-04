// ===== 依赖导入 =====
use glib::timeout_add_local;
use gtk4::prelude::*;
use gtk4::{ApplicationWindow, Image};
use gtk4_layer_shell::{Edge, LayerShell};
use rand::Rng;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use crate::config::{load_animation_path_config, AnimationPathConfig};
use crate::interaction::setup_image_input_region;
use crate::stats::{PetMode, PetRuntimeState, PetStatsService};
use crate::window::position::current_window_left_top;

use super::assets::body_asset_path;
use super::player::{
    AnimationPlayer, DefaultIdlePlayer, DragRaisePlayer, PinchPlayer, ShutdownPlayer,
    SideHideRightMainPlayer, StartupPlayer, TouchPlayer,
};
use super::requests::{
    consume_animation_config_reload_request, consume_requests, set_shutdown_animation_finished,
    AnimationRequests, DRAG_ANIM_END_REQUESTED, DRAG_ANIM_LOOP_REQUESTED,
    DRAG_ANIM_START_REQUESTED, HOVER_ANIM_END_REQUESTED, HOVER_ANIM_START_REQUESTED,
    PINCH_ANIM_END_REQUESTED, PINCH_ANIM_LOOP_REQUESTED,
    PINCH_ANIM_START_REQUESTED, SHUTDOWN_ANIM_REQUESTED, TOUCH_ANIM_BODY_REQUESTED,
    TOUCH_ANIM_HEAD_REQUESTED,
};

// ===== 运行时播放器集合 =====
struct PlayerSet {
    current_mode: PetMode,
    shutdown: ShutdownPlayer,
    drag_raise: DragRaisePlayer,
    pinch: PinchPlayer,
    touch: TouchPlayer,
    startup: StartupPlayer,
    side_hide_right_main: SideHideRightMainPlayer,
    side_hide_right_rise: SideHideRightMainPlayer,
    side_hide_right_trigger_pixel_x: i32,
    side_hide_right_anchor_pixel_x: i32,
    side_hide_right_anchor_pixel_y: i32,
    side_hide_right_trigger_tolerance_px: i32,
    default_idle: DefaultIdlePlayer,
}

impl PlayerSet {
	// 模式切换时重载所有依赖模式的播放器
    fn reload_for_mode(&mut self, before: PetMode, mode: PetMode) {
        self.current_mode = mode;
        self.default_idle.request_mode_switch(before, mode);
        self.drag_raise.reload(mode);
        self.pinch.reload(mode);
        self.touch.reload(mode);
        self.shutdown.reload(mode);
        self.side_hide_right_main.reload(mode);
        self.side_hide_right_rise.reload(mode);
    }

	// 启动时优先首帧（若有 startup）
    fn initial_frame(&mut self) -> Option<PathBuf> {
        if self.startup.is_active() {
            self.startup.peek_first_frame()
        } else {
            self.default_idle.enter()
        }
    }
}

fn map_source_point_to_widget(
    image: &Image,
    current_pixbuf: &Rc<RefCell<Option<gdk_pixbuf::Pixbuf>>>,
    source_x: i32,
    source_y: i32,
) -> (i32, i32) {
    let alloc = image.allocation();
    let widget_w = alloc.width().max(1) as f64;
    let widget_h = alloc.height().max(1) as f64;

    let binding = current_pixbuf.borrow();
    let Some(pixbuf) = binding.as_ref() else {
        return (source_x.max(0), source_y.max(0));
    };

    let pixbuf_w = pixbuf.width().max(1) as f64;
    let pixbuf_h = pixbuf.height().max(1) as f64;

    let mapped_x = ((source_x as f64) * widget_w / pixbuf_w).round() as i32;
    let mapped_y = ((source_y as f64) * widget_h / pixbuf_h).round() as i32;
    (mapped_x.max(0), mapped_y.max(0))
}

fn maybe_trigger_side_hide_right_main(
    players: &mut PlayerSet,
    window: &ApplicationWindow,
    image: &Image,
    current_pixbuf: &Rc<RefCell<Option<gdk_pixbuf::Pixbuf>>>,
) {
    if players.shutdown.is_active()
        || players.drag_raise.is_active()
        || players.pinch.is_active()
        || players.touch.is_active()
        || players.startup.is_active()
        || players.side_hide_right_main.is_active()
        || players.side_hide_right_rise.is_active()
    {
        return;
    }

    let Some(surface) = window.surface() else {
        return;
    };
    let Some(monitor) = surface.display().monitor_at_surface(&surface) else {
        return;
    };
    let monitor_geo = monitor.geometry();
    let monitor_width = monitor_geo.width();

    let (window_left, window_top) = current_window_left_top(window);
    let (threshold_x, _) = map_source_point_to_widget(
        image,
        current_pixbuf,
        players.side_hide_right_trigger_pixel_x,
        players.side_hide_right_anchor_pixel_y,
    );
    let threshold_screen_x = window_left + threshold_x;
    let is_near_right_edge =
        threshold_screen_x >= monitor_width - players.side_hide_right_trigger_tolerance_px;
    if !is_near_right_edge {
        return;
    }

    let (anchor_x, _) = map_source_point_to_widget(
        image,
        current_pixbuf,
        players.side_hide_right_anchor_pixel_x,
        players.side_hide_right_anchor_pixel_y,
    );

    let new_left = monitor_width - anchor_x;
    let new_top = window_top;

    window.set_anchor(Edge::Left, true);
    window.set_anchor(Edge::Top, true);
    window.set_anchor(Edge::Right, false);
    window.set_anchor(Edge::Bottom, false);
    window.set_margin(Edge::Left, new_left);
    window.set_margin(Edge::Top, new_top);

    players.side_hide_right_main.start();
}

struct IdleEventDispatcher {
    interaction_cycle: i32,
    count_nomal: u32,
}

impl IdleEventDispatcher {
    fn new() -> Self {
        Self {
            interaction_cycle: 30,
            count_nomal: 0,
        }
    }

    fn on_timer_elapsed(&mut self, players: &mut PlayerSet, runtime_state: PetRuntimeState, is_press: bool) {
        let is_idel = matches!(runtime_state, PetRuntimeState::Nomal | PetRuntimeState::Work) && !is_press;
        if !is_idel {
            return;
        }

        let mut rnddisplay = (self.interaction_cycle - self.count_nomal as i32).max(20);
        if runtime_state == PetRuntimeState::Work {
            rnddisplay = 2 * rnddisplay + 20;
        }

        if rnddisplay <= 0 {
            return;
        }

        let r = rand::thread_rng().gen_range(0..rnddisplay);

        let triggered = match r {
            3 | 4 | 5 => players.default_idle.trigger_idel(),
            6 => players.default_idle.trigger_state_one(self.count_nomal),
            _ => false,
        };

        if triggered {
            self.count_nomal = 0;
        } else {
            self.count_nomal = self.count_nomal.saturating_add(1);
        }
    }
}

// ===== 请求分发器：将原子请求路由到各播放器 =====
fn dispatch_requests(players: &mut PlayerSet, reqs: AnimationRequests) {
    match reqs.drag {
        DRAG_ANIM_START_REQUESTED => {
            players.shutdown.stop();
            players.side_hide_right_main.stop();
            players.side_hide_right_rise.stop();
            if players.drag_raise.is_playing_end() {
                players.drag_raise.stop();
            }
            players.drag_raise.start(&mut players.pinch, &mut players.touch, &mut players.startup);
            if players.drag_raise.is_active() && !players.drag_raise.is_playing_end() {
                return;
            }
        }
        DRAG_ANIM_LOOP_REQUESTED => {
            players.shutdown.stop();
            players.side_hide_right_main.stop();
            players.side_hide_right_rise.stop();
            if players.drag_raise.is_playing_end() {
                players.drag_raise.stop();
            }
            players.drag_raise.continue_loop(&mut players.pinch, &mut players.touch, &mut players.startup);
            if players.drag_raise.is_active() && !players.drag_raise.is_playing_end() {
                return;
            }
        }
        DRAG_ANIM_END_REQUESTED => {
            players.drag_raise.end();
        }
        _ => {}
    }

    if players.drag_raise.is_active() && !players.drag_raise.is_playing_end() {
        return;
    }

    if reqs.shutdown == SHUTDOWN_ANIM_REQUESTED {
        players.drag_raise.stop();
        players.pinch.stop();
        players.touch.stop();
        players.startup.stop();
        players.side_hide_right_main.stop();
        players.side_hide_right_rise.stop();
        players.shutdown.start();
        return;
    }

    if players.shutdown.is_active() {
        return;
    }

    if players.side_hide_right_main.is_active() {
        let should_interrupt_to_end = matches!(
            reqs.pinch,
            PINCH_ANIM_START_REQUESTED | PINCH_ANIM_LOOP_REQUESTED | PINCH_ANIM_END_REQUESTED
        ) || matches!(reqs.touch, TOUCH_ANIM_HEAD_REQUESTED | TOUCH_ANIM_BODY_REQUESTED);

        if should_interrupt_to_end {
            players.side_hide_right_main.interrupt(false);
        }
        return;
    }

    if players.side_hide_right_rise.is_active() {
        let should_interrupt_to_end = matches!(
            reqs.pinch,
            PINCH_ANIM_START_REQUESTED | PINCH_ANIM_LOOP_REQUESTED | PINCH_ANIM_END_REQUESTED
        ) || matches!(reqs.touch, TOUCH_ANIM_HEAD_REQUESTED | TOUCH_ANIM_BODY_REQUESTED)
            || reqs.hover == HOVER_ANIM_END_REQUESTED;

        if should_interrupt_to_end {
            players.side_hide_right_rise.interrupt(false);
        }
        return;
    }

    match reqs.pinch {
        PINCH_ANIM_START_REQUESTED => {
            if players.drag_raise.is_playing_end() {
                players.drag_raise.stop();
            }
            players.pinch.start(&mut players.touch, &mut players.startup);
        }
        PINCH_ANIM_LOOP_REQUESTED => {
            if players.drag_raise.is_playing_end() {
                players.drag_raise.stop();
            }
            players.pinch.continue_loop(&mut players.touch, &mut players.startup);
        }
        PINCH_ANIM_END_REQUESTED => {
            if players.drag_raise.is_playing_end() {
                players.drag_raise.stop();
            }
            players.pinch.end(&mut players.touch, &mut players.startup);
        }
        _ => {}
    }

    if !players.pinch.is_active() {
        match reqs.touch {
            TOUCH_ANIM_HEAD_REQUESTED => {
                if players.drag_raise.is_playing_end() {
                    players.drag_raise.stop();
                }
                players.touch.start_head(&mut players.startup)
            }
            TOUCH_ANIM_BODY_REQUESTED => {
                if players.drag_raise.is_playing_end() {
                    players.drag_raise.stop();
                }
                players.touch.start_body(&mut players.startup)
            }
            _ => {}
        }

        if reqs.hover == HOVER_ANIM_START_REQUESTED {
            players.side_hide_right_rise.start();
        }
    }
}

// ===== 模式同步：根据数值模式更新播放器素材 =====
fn maybe_update_mode(players: &mut PlayerSet, stats_service: &PetStatsService) {
    if players.startup.is_active() {
        return;
    }

    let next_mode = stats_service.cal_mode();
    if next_mode != players.current_mode {
        let before = players.current_mode;
        players.reload_for_mode(before, next_mode);
    }
}

// ===== 帧推进器：按优先级产出下一帧 =====
fn advance_frame(players: &mut PlayerSet) -> PathBuf {
    if players.shutdown.is_active() {
        if let Some(frame) = players.shutdown.next_frame() {
            return frame;
        }
        players.shutdown.interrupt(true);
        return players.default_idle.enter().unwrap_or_default();
    }

    if players.drag_raise.is_active() {
        if let Some(frame) = players.drag_raise.next_frame() {
            return frame;
        }
        players.drag_raise.interrupt(true);
        return players.default_idle.enter().unwrap_or_default();
    }

    if players.pinch.is_active() {
        if let Some(frame) = players.pinch.next_frame() {
            return frame;
        }
        players.pinch.interrupt(true);
        return players.default_idle.enter().unwrap_or_default();
    }

    if players.touch.is_active() {
        if let Some(frame) = players.touch.next_frame() {
            return frame;
        }
        players.touch.interrupt(true);
        return players.default_idle.enter().unwrap_or_default();
    }

    if players.side_hide_right_rise.is_active() {
        if let Some(frame) = players.side_hide_right_rise.next_frame() {
            return frame;
        }
        players.side_hide_right_rise.interrupt(true);
        return players.default_idle.enter().unwrap_or_default();
    }

    if players.startup.is_active() {
        if let Some(frame) = players.startup.next_frame() {
            return frame;
        }
        players.startup.interrupt(true);
        return players.default_idle.enter().unwrap_or_default();
    }

    if players.side_hide_right_main.is_active() {
        if let Some(frame) = players.side_hide_right_main.next_frame() {
            return frame;
        }
        players.side_hide_right_main.interrupt(true);
        return players.default_idle.enter().unwrap_or_default();
    }

    players
        .default_idle
        .next_frame()
        .or_else(|| players.default_idle.enter())
        .unwrap_or_default()
}

// ===== 根据配置构建播放器集合 =====
fn build_players(
    animation_config: &AnimationPathConfig,
    current_mode: PetMode,
    allow_startup: bool,
) -> Result<PlayerSet, String> {
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
    let side_hide_right_main_root =
        body_asset_path(&animation_config.assets_body_root, &animation_config.side_hide_right_main_root);
    let side_hide_right_rise_root =
        body_asset_path(&animation_config.assets_body_root, &animation_config.side_hide_right_rise_root);

    let mut players = PlayerSet {
        current_mode,
        shutdown: ShutdownPlayer::new(shutdown_root, current_mode),
        drag_raise: DragRaisePlayer::new(drag_raise_dynamic_root, drag_raise_static_root, current_mode),
        pinch: PinchPlayer::new(pinch_root, current_mode),
        touch: TouchPlayer::new(touch_head_root, touch_body_root, current_mode),
        startup: StartupPlayer::new(startup_root, current_mode),
        side_hide_right_main: SideHideRightMainPlayer::new(side_hide_right_main_root, current_mode),
        side_hide_right_rise: SideHideRightMainPlayer::new(side_hide_right_rise_root, current_mode),
        side_hide_right_trigger_pixel_x: animation_config.side_hide_right_trigger_pixel_x,
        side_hide_right_anchor_pixel_x: animation_config.side_hide_right_anchor_pixel_x,
        side_hide_right_anchor_pixel_y: animation_config.side_hide_right_anchor_pixel_y,
        side_hide_right_trigger_tolerance_px: animation_config.side_hide_right_trigger_tolerance_px,
        default_idle: DefaultIdlePlayer::new(animation_config, current_mode)?,
    };

    if !allow_startup {
        players.startup.stop();
    }

    Ok(players)
}

// ===== 动画总入口：创建 image 并启动轮询更新 =====
pub fn load_carousel_images(
    window: &ApplicationWindow,
    current_pixbuf: Rc<RefCell<Option<gdk_pixbuf::Pixbuf>>>,
    stats_service: PetStatsService,
) -> Result<Image, String> {
    let animation_config = load_animation_path_config();
    let current_mode = stats_service.cal_mode();
    let mut players = build_players(&animation_config, current_mode, true)?;

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
    let state_for_logic = state.clone();
    let stats_service_for_logic = stats_service.clone();
    let logic_dispatcher = Rc::new(RefCell::new(IdleEventDispatcher::new()));
    let logic_dispatcher_clone = logic_dispatcher.clone();

    let logic_interval_secs = stats_service.logic_interval_secs();
    let logic_interval_ms = (logic_interval_secs * 1000.0).max(1000.0) as u64;
    timeout_add_local(Duration::from_millis(logic_interval_ms), move || {
        let runtime_state = stats_service_for_logic.runtime_state();
        let mut players = state_for_logic.borrow_mut();
        let is_press = players.drag_raise.is_active()
            || players.pinch.is_active()
            || players.touch.is_active()
            || players.side_hide_right_main.is_active()
            || players.side_hide_right_rise.is_active();
        logic_dispatcher_clone
            .borrow_mut()
            .on_timer_elapsed(&mut players, runtime_state, is_press);
        glib::ControlFlow::Continue
    });


    // 动画 tick：根据当前 DefaultIdlePlayer 类型动态调整 interval
    let tick_state = state.clone();
    let tick_image = image.clone();
    let tick_pixbuf = current_pixbuf.clone();
    let tick_window = window.clone();
    let tick_stats = stats_service.clone();
    fn schedule_tick(
        state: Rc<RefCell<PlayerSet>>,
        image: Image,
        current_pixbuf: Rc<RefCell<Option<gdk_pixbuf::Pixbuf>>>,
        window: ApplicationWindow,
        stats: PetStatsService,
    ) {
        let interval = {
            let players = state.borrow();
            players.default_idle.frame_interval()
        };
        timeout_add_local(Duration::from_millis(interval), move || {
            let next_path = {
                let mut players = state.borrow_mut();
                if consume_animation_config_reload_request() {
                    let latest_config = load_animation_path_config();
                    match build_players(&latest_config, players.current_mode, false) {
                        Ok(next_players) => {
                            *players = next_players;
                        }
                        Err(err) => {
                            eprintln!("动画配置热更新失败，保留当前配置：{}", err);
                        }
                    }
                }
                let reqs = consume_requests();
                maybe_update_mode(&mut players, &stats);
                dispatch_requests(&mut players, reqs);
                maybe_trigger_side_hide_right_main(&mut players, &window, &image, &current_pixbuf);
                advance_frame(&mut players)
            };

            if let Ok(pixbuf) = gdk_pixbuf::Pixbuf::from_file(&next_path) {
                image.set_from_pixbuf(Some(&pixbuf));
                *current_pixbuf.borrow_mut() = Some(pixbuf.clone());
                setup_image_input_region(&window, &image, &pixbuf);
            }

            // 动态重调度下一帧
            schedule_tick(
                state.clone(),
                image.clone(),
                current_pixbuf.clone(),
                window.clone(),
                stats.clone(),
            );
            glib::ControlFlow::Break
        });
    }
    schedule_tick(tick_state, tick_image, tick_pixbuf, tick_window, tick_stats);

    set_shutdown_animation_finished(false);

    Ok(image)
}
