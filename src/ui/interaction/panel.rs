use gtk4::prelude::*;
use gtk4::{
    Align, Application, ApplicationWindow, Box, Button, ComboBoxText, Label, Orientation, Window,
};
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::animation::{
    request_study_book_animation, request_study_paint_animation, request_study_research_animation,
    request_study_stop_animation,
};
use crate::stats::{InteractType, PetStatsService};

const SIDEBAR_WIDTH: i32 = 72;
const SIDEBAR_BUTTON_HEIGHT: i32 = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InteractCategory {
    Study,
    Work,
    Play,
}

impl InteractCategory {
    const ALL: [Self; 3] = [Self::Study, Self::Work, Self::Play];

    fn all() -> &'static [Self] {
        &Self::ALL
    }

    pub fn from_menu_label(label: &str) -> Option<Self> {
        match label {
            "学习" => Some(Self::Study),
            "工作" => Some(Self::Work),
            "玩耍" => Some(Self::Play),
            _ => None,
        }
    }

    fn menu_label(self) -> &'static str {
        match self {
            Self::Study => "学习",
            Self::Work => "工作",
            Self::Play => "玩耍",
        }
    }

    fn panel_title(self) -> &'static str {
        match self {
            Self::Study => "学习互动",
            Self::Work => "工作互动",
            Self::Play => "玩耍互动",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::Study => "提升专注与经验。",
            Self::Work => "努力工作赚取成长资源。",
            Self::Play => "放松心情，提升互动体验。",
        }
    }

    fn interact_type(self) -> InteractType {
        match self {
            Self::Study => InteractType::Study,
            Self::Work => InteractType::Work,
            Self::Play => InteractType::Play,
        }
    }
}

pub struct InteractionPanel {
    window: Window,
    title: Label,
    description: Label,
    status_label: Label,
    current_category: Rc<Cell<InteractCategory>>,
    category_buttons: HashMap<InteractCategory, Button>,
    study_mode_combo: ComboBoxText,
    study_duration_combo: ComboBoxText,
    action_button: Button,
    stop_study_button: Button,
    stats_service: PetStatsService,
    on_after_interact: Rc<dyn Fn()>,
}

impl InteractionPanel {
    pub fn new(
        app: &Application,
        parent_window: &ApplicationWindow,
        category: InteractCategory,
        stats_service: PetStatsService,
        on_after_interact: Rc<dyn Fn()>,
    ) -> Self {
        let window = Window::builder()
            .application(app)
            .title("互动")
            .default_width(420)
            .default_height(320)
            .resizable(true)
            .build();
        window.set_transient_for(Some(parent_window));
        window.set_modal(false);
        window.set_hide_on_close(true);

        let panel_box = Box::new(Orientation::Vertical, 8);
        panel_box.set_margin_top(12);
        panel_box.set_margin_bottom(12);
        panel_box.set_margin_start(12);
        panel_box.set_margin_end(12);

        let title = Label::new(Some("互动"));
        title.set_halign(Align::Center);
        title.set_justify(gtk4::Justification::Center);
        panel_box.append(&title);

        let status_label = Label::new(Some("请选择互动内容"));
        status_label.set_halign(Align::Center);
        status_label.set_justify(gtk4::Justification::Center);
        panel_box.append(&status_label);

        let content_row = Box::new(Orientation::Horizontal, 8);
        content_row.set_hexpand(true);
        content_row.set_vexpand(true);

        let sidebar = Box::new(Orientation::Vertical, 6);
        sidebar.set_valign(Align::Start);
        sidebar.set_halign(Align::Start);
        sidebar.set_hexpand(false);
        sidebar.set_width_request(SIDEBAR_WIDTH);

        let mut category_buttons = HashMap::new();
        for side_category in InteractCategory::all() {
            let button = Button::with_label(side_category.menu_label());
            button.set_halign(Align::Start);
            button.set_hexpand(false);
            button.set_width_request(SIDEBAR_WIDTH);
            button.set_height_request(SIDEBAR_BUTTON_HEIGHT);
            sidebar.append(&button);
            category_buttons.insert(*side_category, button);
        }

        let right_box = Box::new(Orientation::Vertical, 10);
        right_box.set_hexpand(true);
        right_box.set_vexpand(true);

        let description = Label::new(Some(""));
        description.set_halign(Align::Start);
        description.set_wrap(true);
        description.set_xalign(0.0);
        description.set_hexpand(true);
        right_box.append(&description);

        let study_mode_combo = ComboBoxText::new();
        study_mode_combo.append_text("看书");
        study_mode_combo.append_text("画画");
        study_mode_combo.append_text("研究");
        study_mode_combo.set_active(Some(0));
        study_mode_combo.set_halign(Align::Start);
        right_box.append(&study_mode_combo);

        let study_duration_combo = ComboBoxText::new();
        study_duration_combo.append_text("30分钟");
        study_duration_combo.append_text("1小时");
        study_duration_combo.set_active(Some(0));
        study_duration_combo.set_halign(Align::Start);
        right_box.append(&study_duration_combo);

        let action_button = Button::with_label("开始互动");
        action_button.set_halign(Align::Start);

        content_row.append(&sidebar);
        content_row.append(&right_box);
        panel_box.append(&content_row);

        let bottom_actions = Box::new(Orientation::Horizontal, 8);
        bottom_actions.set_halign(Align::End);
        bottom_actions.append(&action_button);

        let stop_study_button = Button::with_label("停止学习");
        bottom_actions.append(&stop_study_button);

        let close_button = Button::with_label("退出");
        bottom_actions.append(&close_button);
        panel_box.append(&bottom_actions);

        window.set_child(Some(&panel_box));

        {
            let window = window.clone();
            close_button.connect_clicked(move |_| {
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

        let panel = Self {
            window,
            title,
            description,
            status_label,
            current_category: Rc::new(Cell::new(category)),
            category_buttons,
            study_mode_combo,
            study_duration_combo,
            action_button,
            stop_study_button,
            stats_service,
            on_after_interact,
        };

        panel.connect_sidebar_handlers();
        panel.connect_action_button();
        panel.connect_stop_study_button();
        panel.switch_category(category);

        panel
    }

    pub fn present(&self) {
        self.window.present();
    }

    pub fn hide(&self) {
        self.window.hide();
    }

    pub fn toggle_category(&self, category: InteractCategory) {
        self.switch_category(category);
        self.present();
    }

    fn switch_category(&self, category: InteractCategory) {
        self.current_category.set(category);
        self.window.set_title(Some(category.panel_title()));
        self.title.set_text(category.panel_title());
        self.description.set_text(category.description());
        self.study_mode_combo
            .set_visible(category == InteractCategory::Study);
        self.study_duration_combo
            .set_visible(category == InteractCategory::Study);
        self.stop_study_button
            .set_visible(category == InteractCategory::Study);
        self.status_label.set_text(&format!("当前选项：{}", category.menu_label()));
        self.update_sidebar_state();
    }

    fn connect_sidebar_handlers(&self) {
        for (category, button) in &self.category_buttons {
            let category = *category;
            let current_category = self.current_category.clone();
            let title = self.title.clone();
            let description = self.description.clone();
            let status_label = self.status_label.clone();
            let window = self.window.clone();
            let category_buttons = self.category_buttons.clone();
            let study_mode_combo = self.study_mode_combo.clone();
            let study_duration_combo = self.study_duration_combo.clone();
            let stop_study_button = self.stop_study_button.clone();

            button.connect_clicked(move |_| {
                current_category.set(category);
                window.set_title(Some(category.panel_title()));
                title.set_text(category.panel_title());
                description.set_text(category.description());
                study_mode_combo.set_visible(category == InteractCategory::Study);
                study_duration_combo.set_visible(category == InteractCategory::Study);
                stop_study_button.set_visible(category == InteractCategory::Study);
                status_label.set_text(&format!("当前选项：{}", category.menu_label()));

                for (button_category, button) in &category_buttons {
                    if *button_category == category {
                        button.add_css_class("suggested-action");
                    } else {
                        button.remove_css_class("suggested-action");
                    }
                }
            });
        }
    }

    fn connect_action_button(&self) {
        let current_category = self.current_category.clone();
        let status_label = self.status_label.clone();
        let stats_service = self.stats_service.clone();
        let on_after_interact = self.on_after_interact.clone();
        let study_mode_combo = self.study_mode_combo.clone();
        let study_duration_combo = self.study_duration_combo.clone();

        self.action_button.connect_clicked(move |_| {
            let category = current_category.get();
            let interact_type = category.interact_type();
            let mut stats_service = stats_service.clone();
            if stats_service.on_interact(interact_type) {
                if category == InteractCategory::Study {
                    let study_mode = study_mode_combo
                        .active_text()
                        .map(|text| text.to_string())
                        .unwrap_or_else(|| "看书".to_string());

                    let duration_secs = match study_duration_combo
                        .active_text()
                        .as_ref()
                        .map(|text| text.as_str())
                    {
                        Some("1小时") => 3600,
                        _ => 1800,
                    };

                    match study_mode.as_str() {
                        "画画" => request_study_paint_animation(duration_secs),
                        "研究" => request_study_research_animation(duration_secs),
                        _ => request_study_book_animation(duration_secs),
                    }

                    let duration_text = if duration_secs == 3600 { "1小时" } else { "30分钟" };
                    status_label.set_text(&format!(
                        "已执行：{}（{}，{}）",
                        category.menu_label(),
                        study_mode,
                        duration_text
                    ));
                } else {
                    status_label.set_text(&format!("已执行：{}", category.menu_label()));
                }
                on_after_interact();
            } else {
                status_label.set_text("当前状态无法互动，请先恢复体力");
            }
        });
    }

    fn connect_stop_study_button(&self) {
        let status_label = self.status_label.clone();
        self.stop_study_button.connect_clicked(move |_| {
            request_study_stop_animation();
            status_label.set_text("已手动停止学习，正在播放收尾动画");
        });
    }

    fn update_sidebar_state(&self) {
        let current = self.current_category.get();
        for (category, button) in &self.category_buttons {
            if *category == current {
                button.add_css_class("suggested-action");
            } else {
                button.remove_css_class("suggested-action");
            }
        }
    }
}
