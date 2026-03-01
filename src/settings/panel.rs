use gtk4::prelude::*;
use gtk4::{Align, Application, ApplicationWindow, Box, Button, CheckButton, Label, Orientation, Window};
use std::cell::RefCell;
use std::rc::Rc;

use super::model::AppSettings;

pub struct SettingsPanel {
    window: Window,
    remember_position_check: CheckButton,
    saved_remember_position: Rc<RefCell<bool>>,
}

impl SettingsPanel {
    pub fn new(
        app: &Application,
        parent_window: &ApplicationWindow,
        initial_settings: AppSettings,
        on_save_settings: Rc<dyn Fn(bool)>,
    ) -> Self {
        let window = Window::builder()
            .application(app)
            .title("设置")
            .default_width(320)
            .default_height(180)
            .resizable(false)
            .build();
        window.set_transient_for(Some(parent_window));

        let panel_box = Box::new(Orientation::Vertical, 8);
        panel_box.set_margin_top(12);
        panel_box.set_margin_bottom(12);
        panel_box.set_margin_start(12);
        panel_box.set_margin_end(12);
        panel_box.set_width_request(280);

        let title = Label::new(Some("设置"));
        title.set_halign(Align::Start);
        panel_box.append(&title);

        let remember_position_check = CheckButton::with_label("位置记忆");
        remember_position_check.set_active(initial_settings.remember_position);
        panel_box.append(&remember_position_check);

        let actions_box = Box::new(Orientation::Horizontal, 8);
        let save_button = Button::with_label("保存");
        let cancel_button = Button::with_label("取消");
        let exit_button = Button::with_label("退出");
        actions_box.append(&save_button);
        actions_box.append(&cancel_button);
        actions_box.append(&exit_button);
        panel_box.append(&actions_box);

        window.set_child(Some(&panel_box));

        let saved_remember_position = Rc::new(RefCell::new(initial_settings.remember_position));

        {
            let window = window.clone();
            let remember_position_check = remember_position_check.clone();
            let saved_remember_position = saved_remember_position.clone();
            let on_save_settings = on_save_settings.clone();
            save_button.connect_clicked(move |_| {
                let remember_position = remember_position_check.is_active();
                on_save_settings(remember_position);
                *saved_remember_position.borrow_mut() = remember_position;
                window.hide();
            });
        }

        {
            let window = window.clone();
            let remember_position_check = remember_position_check.clone();
            let saved_remember_position = saved_remember_position.clone();
            cancel_button.connect_clicked(move |_| {
                remember_position_check.set_active(*saved_remember_position.borrow());
                window.hide();
            });
        }

        {
            let window = window.clone();
            exit_button.connect_clicked(move |_| {
                window.hide();
            });
        }

        {
            let window_for_close = window.clone();
            window.connect_close_request(move |_| {
                window_for_close.hide();
                glib::Propagation::Stop
            });
        }

        Self {
            window,
            remember_position_check,
            saved_remember_position,
        }
    }

    pub fn show(&self) {
        self.remember_position_check
            .set_active(*self.saved_remember_position.borrow());
        self.window.present();
    }

    pub fn hide(&self) {
        self.window.hide();
    }
}
