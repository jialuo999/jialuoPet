// ===== 依赖导入 =====
use gtk4::prelude::*;
use gtk4::{Align, Application, ApplicationWindow, Box, Button, CheckButton, Label, Orientation, Scale, Window};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use super::model::AppSettings;

// ===== 设置面板组件 =====
pub struct SettingsPanel {
    window: Window,
    remember_position_check: CheckButton,
    saved_remember_position: Rc<RefCell<bool>>,
    scale: Scale,
    saved_scale_factor: Rc<Cell<f64>>,
}

impl SettingsPanel {
    pub fn new(
        app: &Application,
        parent_window: &ApplicationWindow,
        initial_settings: AppSettings,
        on_save_settings: Rc<dyn Fn(bool, f64)>,
        on_scale_preview: Rc<dyn Fn(f64)>,
    ) -> Self {
        // ===== 窗口与容器 =====
        let window = Window::builder()
            .application(app)
            .title("设置")
            .default_width(380)
            .default_height(220)
            .resizable(true)
            .build();
        window.set_transient_for(Some(parent_window));

        let panel_box = Box::new(Orientation::Vertical, 8);
        panel_box.set_margin_top(12);
        panel_box.set_margin_bottom(12);
        panel_box.set_margin_start(12);
        panel_box.set_margin_end(12);
        panel_box.set_hexpand(true);
        panel_box.set_vexpand(true);

        let title = Label::new(Some("设置"));
        title.set_halign(Align::Start);
        panel_box.append(&title);

        // ===== 选项区 =====
        let remember_position_check = CheckButton::with_label("位置记忆");
        remember_position_check.set_active(initial_settings.remember_position);
        panel_box.append(&remember_position_check);

        // ===== 缩放设置区 =====
        let scale_title = Label::new(Some("缩放"));
        scale_title.set_halign(Align::Start);
        scale_title.set_margin_top(8);
        panel_box.append(&scale_title);

        let scale_row = Box::new(Orientation::Horizontal, 8);
        let scale = Scale::with_range(Orientation::Horizontal, 50.0, 200.0, 5.0);
        scale.set_value(initial_settings.scale_factor * 100.0);
        scale.set_hexpand(true);
        scale.set_draw_value(false);
        scale_row.append(&scale);

        let pct_text = format!("{}%", (initial_settings.scale_factor * 100.0).round() as i32);
        let scale_value_label = Label::new(Some(&pct_text));
        scale_value_label.set_width_chars(5);
        scale_row.append(&scale_value_label);

        let reset_button = Button::with_label("恢复默认");
        scale_row.append(&reset_button);

        panel_box.append(&scale_row);

        // ===== 布局占位：将操作按钮压到底部 =====
        let spacer = Box::new(Orientation::Vertical, 0);
        spacer.set_vexpand(true);
        panel_box.append(&spacer);

        // ===== 操作按钮区 =====
        let actions_box = Box::new(Orientation::Horizontal, 8);
        actions_box.set_halign(Align::End); // 按钮靠右对齐
        let save_button = Button::with_label("保存");
        let cancel_button = Button::with_label("取消");
        let exit_button = Button::with_label("退出");
        actions_box.append(&save_button);
        actions_box.append(&cancel_button);
        actions_box.append(&exit_button);
        panel_box.append(&actions_box);

        window.set_child(Some(&panel_box));

        let saved_remember_position = Rc::new(RefCell::new(initial_settings.remember_position));
        let saved_scale_factor = Rc::new(Cell::new(initial_settings.scale_factor));

        // ===== 事件绑定：滑块实时预览 =====
        {
            let scale_value_label = scale_value_label.clone();
            let on_scale_preview = on_scale_preview.clone();
            scale.connect_value_changed(move |s| {
                let pct = s.value().round() as i32;
                scale_value_label.set_label(&format!("{}%", pct));
                on_scale_preview(s.value() / 100.0);
            });
        }

        // ===== 事件绑定：恢复默认 =====
        {
            let scale = scale.clone();
            reset_button.connect_clicked(move |_| {
                scale.set_value(100.0);
            });
        }

        // ===== 事件绑定：保存 =====
        {
            let remember_position_check = remember_position_check.clone();
            let saved_remember_position = saved_remember_position.clone();
            let saved_scale_factor = saved_scale_factor.clone();
            let scale = scale.clone();
            let on_save_settings = on_save_settings.clone();
            save_button.connect_clicked(move |_| {
                let remember_position = remember_position_check.is_active();
                let scale_factor = scale.value() / 100.0;
                on_save_settings(remember_position, scale_factor);
                *saved_remember_position.borrow_mut() = remember_position;
                saved_scale_factor.set(scale_factor);
            });
        }

        // ===== 事件绑定：取消（回滚 UI 到上次保存值） =====
        {
            let remember_position_check = remember_position_check.clone();
            let saved_remember_position = saved_remember_position.clone();
            let saved_scale_factor = saved_scale_factor.clone();
            let scale = scale.clone();
            cancel_button.connect_clicked(move |_| {
                remember_position_check.set_active(*saved_remember_position.borrow());
                scale.set_value(saved_scale_factor.get() * 100.0);
            });
        }

        // ===== 事件绑定：退出 =====
        {
            let window = window.clone();
            let saved_scale_factor = saved_scale_factor.clone();
            let scale = scale.clone();
            exit_button.connect_clicked(move |_| {
                scale.set_value(saved_scale_factor.get() * 100.0);
                window.hide();
            });
        }

        // ===== 关闭行为：隐藏窗口并回滚预览 =====
        {
            let window_for_close = window.clone();
            let saved_scale_factor = saved_scale_factor.clone();
            let scale = scale.clone();
            window.connect_close_request(move |_| {
                scale.set_value(saved_scale_factor.get() * 100.0);
                window_for_close.hide();
                glib::Propagation::Stop
            });
        }

        Self {
            window,
            remember_position_check,
            saved_remember_position,
            scale,
            saved_scale_factor,
        }
    }

    // ===== 面板可见性控制 =====
    pub fn show(&self) {
        self.remember_position_check
            .set_active(*self.saved_remember_position.borrow());
        self.scale.set_value(self.saved_scale_factor.get() * 100.0);
        self.window.present();
    }

    pub fn hide(&self) {
        self.scale.set_value(self.saved_scale_factor.get() * 100.0);
        self.window.hide();
    }
}
