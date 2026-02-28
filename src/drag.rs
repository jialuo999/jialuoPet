use gtk4::prelude::*;
use gtk4::{ApplicationWindow, GestureClick, GestureDrag, Image};
use gtk4_layer_shell::{Edge, LayerShell};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use crate::animation::{
    request_drag_raise_animation_end, request_drag_raise_animation_loop,
    request_drag_raise_animation_start,
};
use crate::config::{DRAG_ALLOW_OFFSCREEN, DRAG_LONG_PRESS_MS};

const DRAG_FOCUS_PIXEL_X: i32 = 581;
const DRAG_FOCUS_PIXEL_Y: i32 = 257;

fn focus_pixel_in_widget(
    image: &Image,
    current_pixbuf: &Rc<RefCell<Option<gdk_pixbuf::Pixbuf>>>,
) -> (i32, i32) {
    let alloc = image.allocation();
    let widget_w = alloc.width().max(1) as f64;
    let widget_h = alloc.height().max(1) as f64;

    let binding = current_pixbuf.borrow();
    let Some(pixbuf) = binding.as_ref() else {
        return (DRAG_FOCUS_PIXEL_X, DRAG_FOCUS_PIXEL_Y);
    };

    let pixbuf_w = pixbuf.width().max(1) as f64;
    let pixbuf_h = pixbuf.height().max(1) as f64;

    let mapped_x = ((DRAG_FOCUS_PIXEL_X as f64) * widget_w / pixbuf_w).round() as i32;
    let mapped_y = ((DRAG_FOCUS_PIXEL_Y as f64) * widget_h / pixbuf_h).round() as i32;

    let clamped_x = mapped_x.clamp(0, alloc.width().saturating_sub(1));
    let clamped_y = mapped_y.clamp(0, alloc.height().saturating_sub(1));
    (clamped_x, clamped_y)
}

pub fn setup_long_press_drag(
    window: &ApplicationWindow,
    image: &Image,
    current_pixbuf: Rc<RefCell<Option<gdk_pixbuf::Pixbuf>>>,
) {
    #[derive(Clone, Copy)]
    struct DragState {
        is_pressed: bool,
        press_at: Option<Instant>,
        drag_enabled: bool,
        drag_start_x: f64,
        drag_start_y: f64,
        start_left_margin: i32,
        start_top_margin: i32,
    }

    impl Default for DragState {
        fn default() -> Self {
            Self {
                is_pressed: false,
                press_at: None,
                drag_enabled: false,
                drag_start_x: 0.0,
                drag_start_y: 0.0,
                start_left_margin: 0,
                start_top_margin: 0,
            }
        }
    }

    let state = Rc::new(RefCell::new(DragState::default()));

    let click = GestureClick::new();
    click.set_button(1);
    {
        let state = state.clone();
        click.connect_pressed(move |_, _, _, _| {
            let mut drag_state = state.borrow_mut();
            drag_state.is_pressed = true;
            drag_state.press_at = Some(Instant::now());
            drag_state.drag_enabled = false;
        });
    }
    {
        let state = state.clone();
        click.connect_released(move |_, _, _, _| {
            let mut drag_state = state.borrow_mut();
            let was_dragging = drag_state.drag_enabled;
            drag_state.is_pressed = false;
            drag_state.press_at = None;
            drag_state.drag_enabled = false;
            if was_dragging {
                request_drag_raise_animation_end();
            }
        });
    }
    window.add_controller(click);

    let drag = GestureDrag::new();
    drag.set_button(1);
    {
        let state = state.clone();
        drag.connect_drag_begin(move |_, start_x, start_y| {
            let mut drag_state = state.borrow_mut();
            drag_state.drag_start_x = start_x;
            drag_state.drag_start_y = start_y;
        });
    }
    {
        let state = state.clone();
        let window = window.clone();
        let image = image.clone();
        let current_pixbuf = current_pixbuf.clone();
        drag.connect_drag_update(move |_, offset_x, offset_y| {
            let mut drag_state = state.borrow_mut();
            if !drag_state.is_pressed {
                return;
            }

            if !drag_state.drag_enabled {
                let reached = drag_state
                    .press_at
                    .map(|start| start.elapsed() >= Duration::from_millis(DRAG_LONG_PRESS_MS))
                    .unwrap_or(false);
                if !reached {
                    return;
                }

                let alloc = window.allocation();
                let win_w = alloc.width().max(1);
                let win_h = alloc.height().max(1);

                let (mon_w, mon_h) = window
                    .surface()
                    .and_then(|surface| {
                        let display = surface.display();
                        display.monitor_at_surface(&surface).map(|m| {
                            let geo = m.geometry();
                            (geo.width(), geo.height())
                        })
                    })
                    .unwrap_or((1920, 1080));

                let mut left = if window.is_anchor(Edge::Left) {
                    window.margin(Edge::Left)
                } else if window.is_anchor(Edge::Right) {
                    mon_w - win_w - window.margin(Edge::Right)
                } else {
                    window.margin(Edge::Left)
                };

                let mut top = if window.is_anchor(Edge::Top) {
                    window.margin(Edge::Top)
                } else if window.is_anchor(Edge::Bottom) {
                    mon_h - win_h - window.margin(Edge::Bottom)
                } else {
                    window.margin(Edge::Top)
                };

                if !DRAG_ALLOW_OFFSCREEN {
                    left = left.max(0);
                    top = top.max(0);
                }

                window.set_anchor(Edge::Left, true);
                window.set_anchor(Edge::Top, true);
                window.set_anchor(Edge::Right, false);
                window.set_anchor(Edge::Bottom, false);
                window.set_margin(Edge::Left, left);
                window.set_margin(Edge::Top, top);

                let (focus_x_in_widget, focus_y_in_widget) =
                    focus_pixel_in_widget(&image, &current_pixbuf);

                drag_state.start_left_margin =
                    left + drag_state.drag_start_x.round() as i32 - focus_x_in_widget;
                drag_state.start_top_margin =
                    top + drag_state.drag_start_y.round() as i32 - focus_y_in_widget;

                let (aligned_left, aligned_top) = if DRAG_ALLOW_OFFSCREEN {
                    (drag_state.start_left_margin, drag_state.start_top_margin)
                } else {
                    (drag_state.start_left_margin.max(0), drag_state.start_top_margin.max(0))
                };
                window.set_margin(Edge::Left, aligned_left);
                window.set_margin(Edge::Top, aligned_top);

                drag_state.start_left_margin = aligned_left;
                drag_state.start_top_margin = aligned_top;
                drag_state.drag_enabled = true;
                request_drag_raise_animation_start();
            }

            request_drag_raise_animation_loop();

            let left = drag_state.start_left_margin + offset_x.round() as i32;
            let top = drag_state.start_top_margin + offset_y.round() as i32;
            let (left, top) = if DRAG_ALLOW_OFFSCREEN {
                (left, top)
            } else {
                (left.max(0), top.max(0))
            };
            window.set_margin(Edge::Left, left);
            window.set_margin(Edge::Top, top);
        });
    }
    {
        let state = state.clone();
        drag.connect_drag_end(move |_, _, _| {
            let mut drag_state = state.borrow_mut();
            let was_dragging = drag_state.drag_enabled;
            drag_state.drag_enabled = false;
            if was_dragging {
                request_drag_raise_animation_end();
            }
        });
    }
    window.add_controller(drag);
}