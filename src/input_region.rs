use gtk4::cairo::Region;
use gtk4::prelude::*;
use gtk4::{ApplicationWindow, Box, Button, GestureClick, Image, Orientation, Popover};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use crate::config::{DRAG_LONG_PRESS_MS, INPUT_DEBUG_LOG};
use crate::stats::{InteractType, PetMode, PetStatsService};

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

pub fn setup_image_input_region(
    window: &ApplicationWindow,
    image: &Image,
    pixbuf: &gdk_pixbuf::Pixbuf,
) {
    let Some(surface) = window.surface() else {
        if INPUT_DEBUG_LOG {
            eprintln!("[input-region] skipped: window surface is None");
        }
        return;
    };

    let alloc = image.allocation();
    let (offset_x, offset_y, render_w, render_h) =
        (alloc.x(), alloc.y(), alloc.width(), alloc.height());

    let region = if render_w > 0 && render_h > 0 {
        create_region_from_pixbuf_scaled(pixbuf, offset_x, offset_y, render_w, render_h)
    } else {
        let full = gtk4::cairo::RectangleInt::new(0, 0, pixbuf.width(), pixbuf.height());
        Region::create_rectangle(&full)
    };

    surface.set_input_region(&region);
    if INPUT_DEBUG_LOG {
        eprintln!(
            "[input-region] applied: pixbuf={}x{}, render=({},{} {}x{}), has_alpha={}, region_empty={}",
            pixbuf.width(),
            pixbuf.height(),
            offset_x,
            offset_y,
            render_w,
            render_h,
            pixbuf.has_alpha(),
            region.is_empty()
        );
    }
}

pub fn setup_input_probe(window: &ApplicationWindow, image: &Image) {
    if !INPUT_DEBUG_LOG {
        return;
    }

    let win_click = GestureClick::new();
    win_click.connect_pressed(|_, _, x, y| {
        eprintln!("[probe] window click at ({x:.1}, {y:.1})");
    });
    window.add_controller(win_click);

    let img_click = GestureClick::new();
    img_click.connect_pressed(|_, _, x, y| {
        eprintln!("[probe] image click at ({x:.1}, {y:.1})");
    });
    image.add_controller(img_click);
}

pub fn setup_context_menu(
    image: &Image,
    on_panel_clicked: Rc<dyn Fn(i32, i32)>,
    on_settings_clicked: Rc<dyn Fn()>,
    on_before_menu_popup: Rc<dyn Fn()>,
    on_restart_clicked: Rc<dyn Fn()>,
    on_quit_clicked: Rc<dyn Fn()>,
    is_shutting_down: Rc<dyn Fn() -> bool>,
) {
    let popover = Popover::new();
    popover.set_has_arrow(true);
    popover.set_autohide(false);
    popover.set_parent(image);

    let system_popover = Popover::new();
    system_popover.set_has_arrow(true);
    system_popover.set_autohide(false);
    system_popover.set_parent(image);

    let last_click_pos = Rc::new(RefCell::new((0i32, 0i32)));

    let system_box = Box::new(Orientation::Vertical, 4);
    let settings_button = Button::with_label("设置");
    settings_button.set_halign(gtk4::Align::Fill);
    {
        let popover_for_click = popover.clone();
        let system_popover_for_click = system_popover.clone();
        let on_settings_clicked = on_settings_clicked.clone();
        settings_button.connect_clicked(move |_| {
            popover_for_click.popdown();
            system_popover_for_click.popdown();
            let on_settings_clicked = on_settings_clicked.clone();
            glib::idle_add_local_once(move || {
                on_settings_clicked();
            });
        });
    }

    let restart_button = Button::with_label("重启桌宠");
    restart_button.set_halign(gtk4::Align::Fill);
    {
        let popover_for_click = popover.clone();
        let system_popover_for_click = system_popover.clone();
        let on_restart_clicked = on_restart_clicked.clone();
        restart_button.connect_clicked(move |_| {
            popover_for_click.popdown();
            system_popover_for_click.popdown();
            on_restart_clicked();
        });
    }

    let quit_button = Button::with_label("退出桌宠");
    quit_button.set_halign(gtk4::Align::Fill);
    {
        let popover_for_click = popover.clone();
        let system_popover_for_click = system_popover.clone();
        let on_quit_clicked = on_quit_clicked.clone();
        quit_button.connect_clicked(move |_| {
            popover_for_click.popdown();
            system_popover_for_click.popdown();
            on_quit_clicked();
        });
    }
    system_box.append(&settings_button);
    system_box.append(&restart_button);
    system_box.append(&quit_button);
    system_popover.set_child(Some(&system_box));

    let menu_box = Box::new(Orientation::Vertical, 4);
    for item in ["投喂", "面板", "互动", "系统"] {
        let button = Button::with_label(item);
        button.set_halign(gtk4::Align::Fill);
        if item == "面板" {
            let panel_handler = on_panel_clicked.clone();
            let popover_for_click = popover.clone();
            let last_click_pos = last_click_pos.clone();
            button.connect_clicked(move |_| {
                let (x, y) = *last_click_pos.borrow();
                popover_for_click.popdown();
                let panel_handler = panel_handler.clone();
                glib::idle_add_local_once(move || {
                    panel_handler(x, y);
                });
            });
        } else if item == "系统" {
            let system_popover_for_click = system_popover.clone();
            let last_click_pos = last_click_pos.clone();
            button.connect_clicked(move |_| {
                let (x, y) = *last_click_pos.borrow();
                system_popover_for_click
                    .set_pointing_to(Some(&gdk4::Rectangle::new(x, y, 1, 1)));
                if system_popover_for_click.is_visible() {
                    system_popover_for_click.popdown();
                } else {
                    system_popover_for_click.popup();
                }
            });
        }
        menu_box.append(&button);
    }
    popover.set_child(Some(&menu_box));

    let right_click = GestureClick::new();
    right_click.set_button(3);
    {
        let popover = popover.clone();
        let last_click_pos = last_click_pos.clone();
        let on_before_menu_popup = on_before_menu_popup.clone();
        let system_popover = system_popover.clone();
        let is_shutting_down = is_shutting_down.clone();
        right_click.connect_pressed(move |_, _, x, y| {
            if is_shutting_down() {
                return;
            }
            let xi = x.round() as i32;
            let yi = y.round() as i32;
            *last_click_pos.borrow_mut() = (xi, yi);

            if popover.is_visible() {
                popover.popdown();
                system_popover.popdown();
                return;
            }

            on_before_menu_popup();
            system_popover.popdown();
            popover.set_pointing_to(Some(&gdk4::Rectangle::new(
                xi,
                yi,
                1,
                1,
            )));
            popover.popup();
        });
    }
    image.add_controller(right_click);

    let left_click = GestureClick::new();
    left_click.set_button(1);
    {
        let popover = popover.clone();
        let system_popover = system_popover.clone();
        left_click.connect_pressed(move |_, _, _, _| {
            if popover.is_visible() {
                popover.popdown();
            }
            if system_popover.is_visible() {
                system_popover.popdown();
            }
        });
    }
    image.add_controller(left_click);
}

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

pub fn setup_touch_click_regions(
    image: &Image,
    current_pixbuf: Rc<RefCell<Option<gdk_pixbuf::Pixbuf>>>,
    stats_service: PetStatsService,
    on_head_clicked: Rc<dyn Fn()>,
    on_body_clicked: Rc<dyn Fn()>,
    is_shutting_down: Rc<dyn Fn() -> bool>,
) {
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
                stats_service_for_interact.on_interact(InteractType::TouchHead);
                on_head_clicked();
                return;
            }

            if (body_min_x..=body_max_x).contains(&source_x)
                && (body_min_y..=body_max_y).contains(&source_y)
            {
                let mut stats_service_for_interact = stats_service.clone();
                stats_service_for_interact.on_interact(InteractType::TouchBody);
                on_body_clicked();
            }
        });
    }

    image.add_controller(click);
}

fn create_region_from_pixbuf_scaled(
    pixbuf: &gdk_pixbuf::Pixbuf,
    offset_x: i32,
    offset_y: i32,
    render_w: i32,
    render_h: i32,
) -> Region {
    let src_w = pixbuf.width();
    let src_h = pixbuf.height();

    if !pixbuf.has_alpha() {
        let full = gtk4::cairo::RectangleInt::new(offset_x, offset_y, render_w, render_h);
        return Region::create_rectangle(&full);
    }

    let Some(pixel_bytes) = pixbuf.pixel_bytes() else {
        let full = gtk4::cairo::RectangleInt::new(offset_x, offset_y, render_w, render_h);
        return Region::create_rectangle(&full);
    };

    let bytes = pixel_bytes.as_ref();
    let channels = pixbuf.n_channels() as usize;
    let rowstride = pixbuf.rowstride() as usize;
    let alpha_idx = channels - 1;
    let region = Region::create();

    for dy in 0..render_h {
        let sy = ((dy as i64 * src_h as i64) / render_h as i64) as usize;
        let mut run_start: Option<i32> = None;

        for dx in 0..render_w {
            let sx = ((dx as i64 * src_w as i64) / render_w as i64) as usize;
            let offset = sy * rowstride + sx * channels;
            let alpha = if offset + alpha_idx < bytes.len() {
                bytes[offset + alpha_idx]
            } else {
                0
            };
            match (run_start, alpha > 0) {
                (None, true) => run_start = Some(dx),
                (Some(start), false) => {
                    let rect =
                        gtk4::cairo::RectangleInt::new(offset_x + start, offset_y + dy, dx - start, 1);
                    let _ = region.union_rectangle(&rect);
                    run_start = None;
                }
                _ => {}
            }
        }

        if let Some(start) = run_start {
            let rect =
                gtk4::cairo::RectangleInt::new(offset_x + start, offset_y + dy, render_w - start, 1);
            let _ = region.union_rectangle(&rect);
        }
    }

    if region.is_empty() {
        let full = gtk4::cairo::RectangleInt::new(offset_x, offset_y, render_w, render_h);
        Region::create_rectangle(&full)
    } else {
        region
    }
}