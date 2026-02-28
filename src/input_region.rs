use gtk4::cairo::Region;
use gtk4::prelude::*;
use gtk4::{ApplicationWindow, Box, Button, GestureClick, Image, Orientation, Popover};
use std::cell::RefCell;
use std::rc::Rc;

use crate::config::INPUT_DEBUG_LOG;

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
    on_before_menu_popup: Rc<dyn Fn()>,
    on_restart_clicked: Rc<dyn Fn()>,
    on_quit_clicked: Rc<dyn Fn()>,
) {
    let popover = Popover::new();
    popover.set_has_arrow(true);
    popover.set_autohide(false);
    popover.set_parent(image);

    let system_popover = Popover::new();
    system_popover.set_has_arrow(true);
    system_popover.set_autohide(false);
    system_popover.set_parent(image);

    let system_box = Box::new(Orientation::Vertical, 4);
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
    system_box.append(&restart_button);
    system_box.append(&quit_button);
    system_popover.set_child(Some(&system_box));

    let last_click_pos = Rc::new(RefCell::new((0i32, 0i32)));

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
        right_click.connect_pressed(move |_, _, x, y| {
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