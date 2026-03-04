// ===== 依赖导入 =====
use glib::timeout_add_local;
use gtk4::prelude::*;
use gtk4::{ApplicationWindow, EventControllerMotion, GestureClick, Image, PropagationPhase};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use crate::config::DRAG_LONG_PRESS_MS;
use crate::stats::{InteractType, PetMode, PetStatsService};

// ===== 触摸区域矩形定义（素材坐标系） =====
const TOUCH_HEAD_RECT_X1: i32 = 667;
const TOUCH_HEAD_RECT_Y1: i32 = 113;
const TOUCH_HEAD_RECT_X2: i32 = 373;
const TOUCH_HEAD_RECT_Y2: i32 = 396;

const TOUCH_HEAD_ILL_RECT_X1: i32 = 975;
const TOUCH_HEAD_ILL_RECT_Y1: i32 = 912;
const TOUCH_HEAD_ILL_RECT_X2: i32 = 607;
const TOUCH_HEAD_ILL_RECT_Y2: i32 = 627;

const TOUCH_BODY_RECT_X1: i32 = 634;
const TOUCH_BODY_RECT_Y1: i32 = 944;
const TOUCH_BODY_RECT_X2: i32 = 373;
const TOUCH_BODY_RECT_Y2: i32 = 396;

const TOUCH_BODY_ILL_RECT_X1: i32 = 46;
const TOUCH_BODY_ILL_RECT_Y1: i32 = 934;
const TOUCH_BODY_ILL_RECT_X2: i32 = 607;
const TOUCH_BODY_ILL_RECT_Y2: i32 = 627;
const TOUCH_TAP_MOVE_THRESHOLD: f64 = 8.0;

// ===== 触摸区域数据结构 =====
struct TouchRects {
    head_x1: i32,
    head_y1: i32,
    head_x2: i32,
    head_y2: i32,
    body_x1: i32,
    body_y1: i32,
    body_x2: i32,
    body_y2: i32,
}

// 根据当前模式返回头部/身体区域
fn touch_rects_for_mode(mode: PetMode) -> TouchRects {
    if mode == PetMode::Ill {
        return TouchRects {
            head_x1: TOUCH_HEAD_ILL_RECT_X1,
            head_y1: TOUCH_HEAD_ILL_RECT_Y1,
            head_x2: TOUCH_HEAD_ILL_RECT_X2,
            head_y2: TOUCH_HEAD_ILL_RECT_Y2,
            body_x1: TOUCH_BODY_ILL_RECT_X1,
            body_y1: TOUCH_BODY_ILL_RECT_Y1,
            body_x2: TOUCH_BODY_ILL_RECT_X2,
            body_y2: TOUCH_BODY_ILL_RECT_Y2,
        };
    }

    TouchRects {
        head_x1: TOUCH_HEAD_RECT_X1,
        head_y1: TOUCH_HEAD_RECT_Y1,
        head_x2: TOUCH_HEAD_RECT_X2,
        head_y2: TOUCH_HEAD_RECT_Y2,
        body_x1: TOUCH_BODY_RECT_X1,
        body_y1: TOUCH_BODY_RECT_Y1,
        body_x2: TOUCH_BODY_RECT_X2,
        body_y2: TOUCH_BODY_RECT_Y2,
    }
}

// ===== 坐标换算：控件坐标 -> 素材像素坐标 =====
fn map_point_to_pixbuf(
    image: &Image,
    current_pixbuf: &Rc<RefCell<Option<gdk_pixbuf::Pixbuf>>>,
    pointer_x: f64,
    pointer_y: f64,
) -> Option<(i32, i32)> {
    let alloc = image.allocation();
    if alloc.width() <= 0 || alloc.height() <= 0 {
        return None;
    }

    if pointer_x < 0.0
        || pointer_y < 0.0
        || pointer_x >= alloc.width() as f64
        || pointer_y >= alloc.height() as f64
    {
        return None;
    }

    let binding = current_pixbuf.borrow();
    let pixbuf = binding.as_ref()?;
    let pixbuf_w = pixbuf.width().max(1) as f64;
    let pixbuf_h = pixbuf.height().max(1) as f64;

    let source_x = (pointer_x * pixbuf_w / alloc.width().max(1) as f64).floor() as i32;
    let source_y = (pointer_y * pixbuf_h / alloc.height().max(1) as f64).floor() as i32;
    Some((source_x, source_y))
}

// ===== 点击交互区域装配（头部/身体） =====
pub fn setup_touch_click_regions(
    image: &Image,
    current_pixbuf: Rc<RefCell<Option<gdk_pixbuf::Pixbuf>>>,
    stats_service: PetStatsService,
    on_head_clicked: Rc<dyn Fn()>,
    on_body_clicked: Rc<dyn Fn()>,
    is_shutting_down: Rc<dyn Fn() -> bool>,
) {
    // 记录按下时刻与位置，用于过滤长按和拖动
    #[derive(Default)]
    struct TapState {
        press_x: f64,
        press_y: f64,
        press_at: Option<Instant>,
    }

    let tap_state = Rc::new(RefCell::new(TapState::default()));
    let click = GestureClick::new();
    click.set_button(1);

    {
        let tap_state = tap_state.clone();
        let is_shutting_down = is_shutting_down.clone();
        click.connect_pressed(move |_, _, x, y| {
            if is_shutting_down() {
                return;
            }
            let mut state = tap_state.borrow_mut();
            state.press_x = x;
            state.press_y = y;
            state.press_at = Some(Instant::now());
        });
    }

    {
        let image = image.clone();
        let current_pixbuf = current_pixbuf.clone();
        let tap_state = tap_state.clone();
        let stats_service = stats_service.clone();
        let on_head_clicked = on_head_clicked.clone();
        let on_body_clicked = on_body_clicked.clone();
        let is_shutting_down = is_shutting_down.clone();
        click.connect_released(move |_, _, x, y| {
            if is_shutting_down() {
                return;
            }
            let mut state = tap_state.borrow_mut();
            let Some(press_at) = state.press_at.take() else {
                return;
            };

            if press_at.elapsed().as_millis() as u64 >= DRAG_LONG_PRESS_MS {
                return;
            }

            let dx = x - state.press_x;
            let dy = y - state.press_y;
            if dx.abs() >= TOUCH_TAP_MOVE_THRESHOLD || dy.abs() >= TOUCH_TAP_MOVE_THRESHOLD {
                return;
            }

            let Some((source_x, source_y)) = map_point_to_pixbuf(&image, &current_pixbuf, x, y) else {
                return;
            };

            let mode = stats_service.cal_mode();
            let rects = touch_rects_for_mode(mode);

            let head_min_x = rects.head_x1.min(rects.head_x2);
            let head_max_x = rects.head_x1.max(rects.head_x2);
            let head_min_y = rects.head_y1.min(rects.head_y2);
            let head_max_y = rects.head_y1.max(rects.head_y2);

            let body_min_x = rects.body_x1.min(rects.body_x2);
            let body_max_x = rects.body_x1.max(rects.body_x2);
            let body_min_y = rects.body_y1.min(rects.body_y2);
            let body_max_y = rects.body_y1.max(rects.body_y2);

            if (head_min_x..=head_max_x).contains(&source_x)
                && (head_min_y..=head_max_y).contains(&source_y)
            {
                let mut stats_service_for_interact = stats_service.clone();
                if stats_service_for_interact.on_interact(InteractType::TouchHead) {
                    on_head_clicked();
                }
                return;
            }

            if (body_min_x..=body_max_x).contains(&source_x)
                && (body_min_y..=body_max_y).contains(&source_y)
            {
                let mut stats_service_for_interact = stats_service.clone();
                if stats_service_for_interact.on_interact(InteractType::TouchBody) {
                    on_body_clicked();
                }
            }
        });
    }

    image.add_controller(click);
}

pub fn setup_hover_regions(
    window: &ApplicationWindow,
    on_hover_entered: Rc<dyn Fn()>,
    on_hover_left: Rc<dyn Fn()>,
    is_shutting_down: Rc<dyn Fn() -> bool>,
) {
    // 在 niri + layer-shell 环境中，挂在 Image 上的 enter/leave 可能丢失。
    // 这里改为挂在 Window 上，并使用 motion 作为兜底信号。
    let motion = EventControllerMotion::new();
    motion.set_propagation_phase(PropagationPhase::Capture);
    let is_hovering = Rc::new(RefCell::new(false));

    {
        let on_hover_entered = on_hover_entered.clone();
        let is_shutting_down = is_shutting_down.clone();
        let is_hovering = is_hovering.clone();
        motion.connect_enter(move |_, _, _| {
            if is_shutting_down() {
                return;
            }
            if *is_hovering.borrow() {
                return;
            }
            *is_hovering.borrow_mut() = true;
            on_hover_entered();
        });
    }

    {
        let on_hover_entered = on_hover_entered.clone();
        let is_shutting_down = is_shutting_down.clone();
        let is_hovering = is_hovering.clone();
        motion.connect_motion(move |_, _, _| {
            if is_shutting_down() {
                return;
            }
            // 兜底：若 enter 事件未到达，但窗口已收到 motion，则视为“已悬浮”。
            if *is_hovering.borrow() {
                return;
            }
            *is_hovering.borrow_mut() = true;
            on_hover_entered();
        });
    }

    {
        let on_hover_left = on_hover_left.clone();
        let is_shutting_down = is_shutting_down.clone();
        let is_hovering = is_hovering.clone();
        motion.connect_leave(move |_| {
            if is_shutting_down() {
                return;
            }
            if !*is_hovering.borrow() {
                return;
            }
            *is_hovering.borrow_mut() = false;
            on_hover_left();
        });
    }

    window.add_controller(motion);

    // 二级兜底（niri 场景）：部分 compositor 下 enter/leave 仍可能不稳定，
    // 通过周期性查询“当前指针所在 surface”来补发 enter/leave。
    let window_weak = window.downgrade();
    let is_hovering_for_poll = is_hovering.clone();
    let on_hover_entered_for_poll = on_hover_entered.clone();
    let on_hover_left_for_poll = on_hover_left.clone();
    let is_shutting_down_for_poll = is_shutting_down.clone();
    timeout_add_local(Duration::from_millis(120), move || {
        if is_shutting_down_for_poll() {
            return glib::ControlFlow::Continue;
        }

        let Some(window) = window_weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        let Some(window_surface) = window.surface() else {
            return glib::ControlFlow::Continue;
        };

        let is_pointer_on_this_surface = gdk4::Display::default()
            .and_then(|display| display.default_seat())
            .and_then(|seat| seat.pointer())
            .map(|pointer| pointer.surface_at_position())
            .and_then(|(surface, _, _)| surface)
            .map(|surface| surface == window_surface)
            .unwrap_or(false);

        let mut hovering = is_hovering_for_poll.borrow_mut();
        if is_pointer_on_this_surface && !*hovering {
            *hovering = true;
            on_hover_entered_for_poll();
        } else if !is_pointer_on_this_surface && *hovering {
            *hovering = false;
            on_hover_left_for_poll();
        }

        glib::ControlFlow::Continue
    });
}
