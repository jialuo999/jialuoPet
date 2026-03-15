// ===== 依赖导入 =====
use gtk4::prelude::*;
use gtk4::{Box, Button, GestureClick, Image, Orientation, Popover};
use std::cell::RefCell;
use std::rc::Rc;

// ===== 右键菜单装配（主菜单 + 系统子菜单） =====
pub fn setup_context_menu(
    image: &Image,
    on_panel_clicked: Rc<dyn Fn(i32, i32)>,
    on_feed_panel_clicked: Rc<dyn Fn(&'static str)>,
    on_interact_clicked: Rc<dyn Fn(&'static str)>,
    on_settings_clicked: Rc<dyn Fn()>,
    on_before_menu_popup: Rc<dyn Fn()>,
    on_restart_clicked: Rc<dyn Fn()>,
    on_quit_clicked: Rc<dyn Fn()>,
    is_shutting_down: Rc<dyn Fn() -> bool>,
) {
    // 主菜单 popover
    let popover = Popover::new();
    popover.set_has_arrow(true);
    popover.set_autohide(false);
    popover.set_parent(image);

    let system_popover = Popover::new();
    system_popover.set_has_arrow(true);
    system_popover.set_autohide(false);
    system_popover.set_parent(image);

    let feed_popover = Popover::new();
    feed_popover.set_has_arrow(true);
    feed_popover.set_autohide(false);
    feed_popover.set_parent(image);

    let interact_popover = Popover::new();
    interact_popover.set_has_arrow(true);
    interact_popover.set_autohide(false);
    interact_popover.set_parent(image);

    let last_click_pos = Rc::new(RefCell::new((0i32, 0i32)));

    // 系统子菜单：设置/重启/退出
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

    // 投喂子菜单：主食/饮品/零食/礼物/药物/功能
    let feed_box = Box::new(Orientation::Vertical, 4);
    for item in ["主食", "饮品", "零食", "礼物", "药物", "功能"] {
        let button = Button::with_label(item);
        button.set_halign(gtk4::Align::Fill);
        let popover_for_click = popover.clone();
        let system_popover_for_click = system_popover.clone();
        let feed_popover_for_click = feed_popover.clone();
        let on_feed_panel_clicked = on_feed_panel_clicked.clone();
        button.connect_clicked(move |_| {
            popover_for_click.popdown();
            system_popover_for_click.popdown();
            feed_popover_for_click.popdown();

            let on_feed_panel_clicked = on_feed_panel_clicked.clone();
            glib::idle_add_local_once(move || {
                on_feed_panel_clicked(item);
            });
        });
        feed_box.append(&button);
    }
    feed_popover.set_child(Some(&feed_box));

    // 互动子菜单：学习/工作/玩耍
    let interact_box = Box::new(Orientation::Vertical, 4);
    for item in ["学习", "工作", "玩耍"] {
        let button = Button::with_label(item);
        button.set_halign(gtk4::Align::Fill);
        let popover_for_click = popover.clone();
        let system_popover_for_click = system_popover.clone();
        let feed_popover_for_click = feed_popover.clone();
        let interact_popover_for_click = interact_popover.clone();
        let on_interact_clicked = on_interact_clicked.clone();
        button.connect_clicked(move |_| {
            popover_for_click.popdown();
            system_popover_for_click.popdown();
            feed_popover_for_click.popdown();
            interact_popover_for_click.popdown();

            let on_interact_clicked = on_interact_clicked.clone();
            glib::idle_add_local_once(move || {
                on_interact_clicked(item);
            });
        });
        interact_box.append(&button);
    }
    interact_popover.set_child(Some(&interact_box));

    // 主菜单：投喂/面板/互动/系统（目前实现面板与系统）
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
        } else if item == "投喂" {
            let feed_popover_for_click = feed_popover.clone();
            let system_popover_for_click = system_popover.clone();
            let interact_popover_for_click = interact_popover.clone();
            let last_click_pos = last_click_pos.clone();
            button.connect_clicked(move |_| {
                let (x, y) = *last_click_pos.borrow();
                system_popover_for_click.popdown();
                interact_popover_for_click.popdown();
                feed_popover_for_click
                    .set_pointing_to(Some(&gdk4::Rectangle::new(x, y, 1, 1)));
                if feed_popover_for_click.is_visible() {
                    feed_popover_for_click.popdown();
                } else {
                    feed_popover_for_click.popup();
                }
            });
        } else if item == "系统" {
            let system_popover_for_click = system_popover.clone();
            let feed_popover_for_click = feed_popover.clone();
            let interact_popover_for_click = interact_popover.clone();
            let last_click_pos = last_click_pos.clone();
            button.connect_clicked(move |_| {
                let (x, y) = *last_click_pos.borrow();
                feed_popover_for_click.popdown();
                interact_popover_for_click.popdown();
                system_popover_for_click
                    .set_pointing_to(Some(&gdk4::Rectangle::new(x, y, 1, 1)));
                if system_popover_for_click.is_visible() {
                    system_popover_for_click.popdown();
                } else {
                    system_popover_for_click.popup();
                }
            });
        } else if item == "互动" {
            let system_popover_for_click = system_popover.clone();
            let feed_popover_for_click = feed_popover.clone();
            let interact_popover_for_click = interact_popover.clone();
            let last_click_pos = last_click_pos.clone();
            button.connect_clicked(move |_| {
                let (x, y) = *last_click_pos.borrow();
                feed_popover_for_click.popdown();
                system_popover_for_click.popdown();
                interact_popover_for_click
                    .set_pointing_to(Some(&gdk4::Rectangle::new(x, y, 1, 1)));
                if interact_popover_for_click.is_visible() {
                    interact_popover_for_click.popdown();
                } else {
                    interact_popover_for_click.popup();
                }
            });
        }
        menu_box.append(&button);
    }
    popover.set_child(Some(&menu_box));

    // 右键：弹出/收起菜单
    let right_click = GestureClick::new();
    right_click.set_button(3);
    {
        let popover = popover.clone();
        let last_click_pos = last_click_pos.clone();
        let on_before_menu_popup = on_before_menu_popup.clone();
        let system_popover = system_popover.clone();
        let feed_popover = feed_popover.clone();
        let interact_popover = interact_popover.clone();
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
                feed_popover.popdown();
                interact_popover.popdown();
                return;
            }

            on_before_menu_popup();
            system_popover.popdown();
            feed_popover.popdown();
            interact_popover.popdown();
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

    // 左键：点击外部时关闭菜单
    let left_click = GestureClick::new();
    left_click.set_button(1);
    {
        let popover = popover.clone();
        let system_popover = system_popover.clone();
        let feed_popover = feed_popover.clone();
        let interact_popover = interact_popover.clone();
        left_click.connect_pressed(move |_, _, _, _| {
            if popover.is_visible() {
                popover.popdown();
            }
            if system_popover.is_visible() {
                system_popover.popdown();
            }
            if feed_popover.is_visible() {
                feed_popover.popdown();
            }
            if interact_popover.is_visible() {
                interact_popover.popdown();
            }
        });
    }
    image.add_controller(left_click);
}
