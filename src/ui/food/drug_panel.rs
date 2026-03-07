use gtk4::prelude::*;
use gtk4::{
    Align, Application, ApplicationWindow, Box, Button, CssProvider, FlowBox, Image, Label,
    Orientation, ScrolledWindow, SelectionMode, Window, STYLE_PROVIDER_PRIORITY_APPLICATION,
};
use std::cell::Cell;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::rc::Rc;

use crate::stats::food::{ItemDef, ItemEffects, ItemKind};
use crate::stats::PetStatsService;

// 图片尺寸：修改这里可调整每个单元格中图片显示大小（像素）。
const THUMBNAIL_SIZE: i32 = 48;
// 统一单元格尺寸：在这里修改每个物品格子的宽高。
const ITEM_CELL_WIDTH: i32 = 68;
const ITEM_CELL_HEIGHT: i32 = 62;
// 文字尺寸：修改这里可调整名称最多显示字符数与字号（pt）。
const ITEM_NAME_VIEW_CHARS: usize = 6;
const ITEM_NAME_FONT_PT: i32 = 9;
const ITEM_NAME_VIEWPORT_WIDTH: i32 = 72;
const ITEM_NAME_VIEWPORT_HEIGHT: i32 = 18;
const SIDEBAR_WIDTH: i32 = 28;
const SIDEBAR_BUTTON_HEIGHT: i32 = 44;
const CONTENT_ROW_SPACING: i32 = 8;
const FLOWBOX_COLUMN_SPACING: u32 = 3;
const FLOWBOX_ROW_SPACING: u32 = 3;
// FlowBox 默认每行子项数量有上限（通常为 7），显式放宽避免窗口变宽后列数不再增加。
const FLOWBOX_MAX_COLUMNS: u32 = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FeedCategory {
    Meal,
    Drink,
    Snack,
    Gift,
    Drug,
    Functional,
}

impl FeedCategory {
    const ALL: [Self; 6] = [
        Self::Meal,
        Self::Drink,
        Self::Snack,
        Self::Gift,
        Self::Drug,
        Self::Functional,
    ];

    fn all() -> &'static [Self] {
        &Self::ALL
    }

    pub fn from_menu_label(label: &str) -> Option<Self> {
        match label {
            "主食" => Some(Self::Meal),
            "饮品" => Some(Self::Drink),
            "零食" => Some(Self::Snack),
            "礼物" => Some(Self::Gift),
            "药物" => Some(Self::Drug),
            "功能" => Some(Self::Functional),
            _ => None,
        }
    }

    fn menu_label(self) -> &'static str {
        match self {
            Self::Meal => "主食",
            Self::Drink => "饮品",
            Self::Snack => "零食",
            Self::Gift => "礼物",
            Self::Drug => "药物",
            Self::Functional => "功能",
        }
    }

    fn panel_title(self) -> &'static str {
        match self {
            Self::Meal => "主食面板",
            Self::Drink => "饮品面板",
            Self::Snack => "零食面板",
            Self::Gift => "礼物面板",
            Self::Drug => "药物面板",
            Self::Functional => "功能面板",
        }
    }

    fn image_dir(self) -> &'static str {
        match self {
            Self::Meal => "assets/image/food/meal",
            Self::Drink => "assets/image/food/drink",
            Self::Snack => "assets/image/food/snack",
            Self::Gift => "assets/image/food/gift",
            Self::Drug => "assets/image/food/drug",
            Self::Functional => "assets/image/food/functional",
        }
    }

    fn instruct_files(self) -> &'static [&'static str] {
        match self {
            Self::Meal => &["assets/Instruct/food.lps", "assets/Instruct/timelimit.lps"],
            Self::Drink => &[
                "assets/Instruct/food.lps",
                "assets/Instruct/moredrink.lps",
                "assets/Instruct/timelimit.lps",
            ],
            Self::Snack => &["assets/Instruct/food.lps", "assets/Instruct/timelimit.lps"],
            Self::Gift => &["assets/Instruct/gift.lps", "assets/Instruct/timelimit.lps"],
            Self::Drug => &["assets/Instruct/drug.lps"],
            Self::Functional => &["assets/Instruct/food.lps", "assets/Instruct/timelimit.lps"],
        }
    }

    fn type_filter(self) -> &'static str {
        match self {
            Self::Meal => "Meal",
            Self::Drink => "Drink",
            Self::Snack => "Snack",
            Self::Gift => "Gift",
            Self::Drug => "Drug",
            Self::Functional => "Functional",
        }
    }
}

pub struct FeedPanel {
    window: Window,
    title: Label,
    status_label: Label,
    flow: FlowBox,
    current_category: Rc<Cell<FeedCategory>>,
    stats_service: PetStatsService,
    on_after_use: Rc<dyn Fn()>,
    category_buttons: HashMap<FeedCategory, Button>,
}

impl FeedPanel {
    pub fn new(
        app: &Application,
        parent_window: &ApplicationWindow,
        category: FeedCategory,
        stats_service: PetStatsService,
        on_after_use: Rc<dyn Fn()>,
    ) -> Self {
        let window = Window::builder()
            .application(app)
            .title(category.panel_title())
            .default_width(520)
            .default_height(420)
            .resizable(true)
            .build();
        window.set_transient_for(Some(parent_window));
        window.set_modal(false);
        window.set_hide_on_close(true);

        // 固定单元格在普通/悬停/按下状态下的尺寸与内边距，避免视觉尺寸跳变。
        let css_provider = CssProvider::new();
        css_provider.load_from_data(
            &format!(
                ".feed-item-cell,\n\
                 .feed-item-cell:hover,\n\
                 .feed-item-cell:active,\n\
                 .feed-item-cell:checked {{\n\
                    min-width: {}px;\n\
                    max-width: {}px;\n\
                    min-height: {}px;\n\
                    max-height: {}px;\n\
                    padding: 0;\n\
                    margin: 0;\n\
                }}\n\
                .feed-sidebar-button,\n\
                .feed-sidebar-button:hover,\n\
                .feed-sidebar-button:active,\n\
                .feed-sidebar-button:checked {{\n\
                    min-width: {}px;\n\
                    max-width: {}px;\n\
                    padding-left: 2px;\n\
                    padding-right: 2px;\n\
                }}",
                ITEM_CELL_WIDTH,
                ITEM_CELL_WIDTH,
                ITEM_CELL_HEIGHT,
                ITEM_CELL_HEIGHT,
                SIDEBAR_WIDTH,
                SIDEBAR_WIDTH
            ),
        );
        window
            .style_context()
            .add_provider(&css_provider, STYLE_PROVIDER_PRIORITY_APPLICATION);

        let panel_box = Box::new(Orientation::Vertical, 8);
        panel_box.set_margin_top(12);
        panel_box.set_margin_bottom(12);
        panel_box.set_margin_start(12);
        panel_box.set_margin_end(12);

        let title = Label::new(Some(category.panel_title()));
        title.set_halign(Align::Center);
        title.set_justify(gtk4::Justification::Center);
        panel_box.append(&title);

        let status_label = Label::new(Some("点击物品可立即生效"));
        status_label.set_halign(Align::Center);
        status_label.set_justify(gtk4::Justification::Center);
        panel_box.append(&status_label);

        let content_row = Box::new(Orientation::Horizontal, CONTENT_ROW_SPACING);
        content_row.set_hexpand(true);
        content_row.set_vexpand(true);

        let scroll = ScrolledWindow::new();
        scroll.set_hexpand(true);
        scroll.set_vexpand(true);

        let flow = FlowBox::new();
        flow.set_column_spacing(FLOWBOX_COLUMN_SPACING);
        flow.set_row_spacing(FLOWBOX_ROW_SPACING);
        flow.set_selection_mode(SelectionMode::None);
        // 固定 FlowBox 子项为统一网格尺寸，避免文本自然宽度影响每个单元格大小。
        flow.set_homogeneous(true);
        flow.set_valign(Align::Start);
        flow.set_vexpand(false);
        flow.set_max_children_per_line(FLOWBOX_MAX_COLUMNS);

        scroll.set_child(Some(&flow));

        let sidebar = Box::new(Orientation::Vertical, 6);
        sidebar.set_valign(Align::Start);
        sidebar.set_halign(Align::Start);
        sidebar.set_hexpand(false);
        sidebar.set_width_request(SIDEBAR_WIDTH);

        let mut category_buttons = HashMap::new();
        for side_category in FeedCategory::all() {
            let button = Button::with_label(side_category.menu_label());
            button.add_css_class("feed-sidebar-button");
            button.set_halign(Align::Start);
            button.set_hexpand(false);
            button.set_width_request(SIDEBAR_WIDTH);
            button.set_height_request(SIDEBAR_BUTTON_HEIGHT);
            sidebar.append(&button);
            category_buttons.insert(*side_category, button);
        }

        content_row.append(&sidebar);
        content_row.append(&scroll);
        panel_box.append(&content_row);


        let actions_box = Box::new(Orientation::Horizontal, 8);
        actions_box.set_halign(Align::End);
        let close_button = Button::with_label("退出");
        actions_box.append(&close_button);
        panel_box.append(&actions_box);

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
            status_label,
            flow,
            current_category: Rc::new(Cell::new(category)),
            stats_service,
            on_after_use,
            category_buttons,
        };

        panel.connect_sidebar_handlers();
        panel.switch_category(category);

        panel
    }

    pub fn present(&self) {
        self.window.present();
    }

    pub fn toggle_category(&self, category: FeedCategory) {
        if self.window.is_visible() && self.current_category.get() == category {
            self.window.hide();
            return;
        }

        self.switch_category(category);
        self.present();
    }

    pub fn switch_category(&self, category: FeedCategory) {
        self.current_category.set(category);
        self.window.set_title(Some(category.panel_title()));
        self.title.set_text(category.panel_title());
        self.status_label.set_text("点击物品可立即生效");
        self.refresh_sidebar_state();
        self.reload_items_for(category);
    }

    pub fn hide(&self) {
        self.window.hide();
    }

    fn connect_sidebar_handlers(&self) {
        for side_category in FeedCategory::all() {
            if let Some(button) = self.category_buttons.get(side_category) {
                let panel = self.clone_ref();
                let target = *side_category;
                button.connect_clicked(move |_| {
                    panel.switch_category(target);
                });
            }
        }
    }

    fn refresh_sidebar_state(&self) {
        let current = self.current_category.get();
        for side_category in FeedCategory::all() {
            if let Some(button) = self.category_buttons.get(side_category) {
                if *side_category == current {
                    button.add_css_class("suggested-action");
                } else {
                    button.remove_css_class("suggested-action");
                }
            }
        }
    }

    fn reload_items_for(&self, category: FeedCategory) {
        while let Some(child) = self.flow.first_child() {
            self.flow.remove(&child);
        }

        let image_paths = list_png_files(category.image_dir());
        let item_map = load_items(category);

        if image_paths.is_empty() {
            let empty = Label::new(Some("未找到图片资源"));
            empty.set_halign(Align::Center);
            self.flow.insert(&empty, -1);
            return;
        }

        for path in &image_paths {
            let cell = build_item_cell(
                path,
                &item_map,
                self.stats_service.clone(),
                self.on_after_use.clone(),
                &self.status_label,
            );
            self.flow.insert(&cell, -1);
        }
    }

    fn clone_ref(&self) -> Self {
        Self {
            window: self.window.clone(),
            title: self.title.clone(),
            status_label: self.status_label.clone(),
            flow: self.flow.clone(),
            current_category: self.current_category.clone(),
            stats_service: self.stats_service.clone(),
            on_after_use: self.on_after_use.clone(),
            category_buttons: self.category_buttons.clone(),
        }
    }
}

fn list_png_files(dir: &str) -> Vec<String> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let is_png = path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("png"))
                .unwrap_or(false);
            if is_png {
                files.push(path.to_string_lossy().to_string());
            }
        }
    }

    files.sort();
    files
}

fn build_item_cell(
    path: &str,
    item_map: &HashMap<String, ItemDef>,
    stats_service: PetStatsService,
    on_after_use: Rc<dyn Fn()>,
    status_label: &Label,
) -> Button {
    let button = Button::new();
    button.add_css_class("feed-item-cell");
    button.set_width_request(ITEM_CELL_WIDTH);
    button.set_height_request(ITEM_CELL_HEIGHT);
    button.set_hexpand(false);
    button.set_vexpand(false);
    button.set_halign(Align::Center);
    button.set_valign(Align::Start);

    let content = Box::new(Orientation::Vertical, 4);
    content.set_width_request(ITEM_CELL_WIDTH);
    content.set_height_request(ITEM_CELL_HEIGHT);
    content.set_halign(Align::Center);
    content.set_valign(Align::Center);

    let image = Image::from_file(path);
    image.set_pixel_size(THUMBNAIL_SIZE);
    image.set_halign(Align::Center);
    content.append(&image);

    let filename = Path::new(path)
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
        .to_string();
    let name_viewport = Box::new(Orientation::Horizontal, 0);
    name_viewport.set_width_request(ITEM_NAME_VIEWPORT_WIDTH);
    name_viewport.set_height_request(ITEM_NAME_VIEWPORT_HEIGHT);
    name_viewport.set_halign(Align::Center);
    name_viewport.set_valign(Align::Center);
    let name_label = Label::new(Some(&filename));
    name_label.set_halign(Align::Center);
    name_label.set_valign(Align::Center);
    name_label.set_wrap(false);
    name_label.set_single_line_mode(true);
    name_label.set_width_chars(ITEM_NAME_VIEW_CHARS as i32);
    name_label.set_max_width_chars(ITEM_NAME_VIEW_CHARS as i32);
    name_label.set_tooltip_text(Some(&filename));
    set_label_text_with_font(&name_label, &filename);
    name_viewport.append(&name_label);
    content.append(&name_viewport);

    button.set_child(Some(&content));

    if let Some(item_def) = item_map.get(&filename).cloned() {
        let status_label = status_label.clone();
        button.connect_clicked(move |_| {
            let mut service = stats_service.clone();
            if service.on_use_item(&item_def) {
                status_label.set_text(&format!("已使用：{}", item_def.id));
                on_after_use();
            } else {
                status_label.set_text(&format!("金钱不足：{} 需要 {}", item_def.id, item_def.price));
            }
        });
    } else {
        let status_label = status_label.clone();
        let item_name = filename.clone();
        button.connect_clicked(move |_| {
            status_label.set_text(&format!("未在配置中找到：{}", item_name));
        });
    }

    button
}

fn load_items(category: FeedCategory) -> HashMap<String, ItemDef> {
    let mut items = HashMap::new();
    let type_filter = category.type_filter();

    for file in category.instruct_files() {
        let Ok(content) = fs::read_to_string(file) else {
            continue;
        };

        for line in content.lines() {
            let trimmed = line.trim();
            let Some(start) = trimmed.find("food:") else {
                continue;
            };
            let normalized = &trimmed[start..];
            if let Some(item) = parse_item_line(normalized, type_filter) {
                items.insert(item.id.clone(), item);
            }
        }
    }

    items
}

fn set_label_text_with_font(label: &Label, text: &str) {
    let font_size = ITEM_NAME_FONT_PT * gtk4::pango::SCALE;
    let escaped_text = glib::markup_escape_text(text);
    label.set_markup(&format!("<span size=\"{}\">{}</span>", font_size, escaped_text));
}

fn parse_item_line(line: &str, type_filter: &str) -> Option<ItemDef> {
    let mut fields: HashMap<String, String> = HashMap::new();
    for part in line.split('|') {
        let piece = part.trim();
        if let Some((key, value)) = piece.split_once('#') {
            let key = key.trim();
            if key.is_empty() {
                continue;
            }

            let value = value.trim_end_matches(':').trim();
            fields.insert(key.to_string(), value.to_string());
        }
    }

    let item_type = fields.get("type")?.trim();
    if item_type != type_filter {
        return None;
    }

    let name = fields.get("name")?.trim().to_string();
    let mut effects = ItemEffects::default();
    effects.exp = parse_num(&fields, "Exp");
    effects.stamina = parse_num(&fields, "Strength");
    effects.satiety = parse_num(&fields, "StrengthFood");
    effects.thirst = parse_num(&fields, "StrengthDrink");
    effects.health = parse_num(&fields, "Health");
    effects.mood = parse_num(&fields, "Feeling");
    effects.likability = parse_num(&fields, "Likability");

    let price = parse_num(&fields, "price").round().max(0.0) as u32;

    Some(ItemDef {
        id: name,
        kind: parse_kind(type_filter),
        price,
        stack_limit: 99,
        effects,
    })
}

fn parse_kind(kind: &str) -> ItemKind {
    match kind {
        "Meal" => ItemKind::Staple,
        "Drink" => ItemKind::Drink,
        "Snack" => ItemKind::Snack,
        "Gift" => ItemKind::Gift,
        "Drug" => ItemKind::Drug,
        "Functional" => ItemKind::Functional,
        _ => ItemKind::Snack,
    }
}

fn parse_num(fields: &HashMap<String, String>, key: &str) -> f64 {
    fields
        .get(key)
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(0.0)
}
