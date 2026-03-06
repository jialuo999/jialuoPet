use gtk4::prelude::*;
use gtk4::{
    Align, Application, ApplicationWindow, Box, Button, Grid, Image, Label, Orientation,
    ScrolledWindow, Window,
};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::rc::Rc;

use crate::stats::food::{ItemDef, ItemEffects, ItemKind};
use crate::stats::PetStatsService;

const THUMBNAIL_SIZE: i32 = 64;
const GRID_COLUMNS: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedCategory {
    Meal,
    Drink,
    Snack,
    Gift,
    Drug,
    Functional,
}

impl FeedCategory {
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

        let panel_box = Box::new(Orientation::Vertical, 8);
        panel_box.set_margin_top(12);
        panel_box.set_margin_bottom(12);
        panel_box.set_margin_start(12);
        panel_box.set_margin_end(12);
        panel_box.set_width_request(480);

        let title = Label::new(Some(category.panel_title()));
        title.set_halign(Align::Start);
        panel_box.append(&title);

        let status_label = Label::new(Some("点击物品可立即生效"));
        status_label.set_halign(Align::Start);
        panel_box.append(&status_label);

        let scroll = ScrolledWindow::new();
        scroll.set_hexpand(true);
        scroll.set_vexpand(true);

        let grid = Grid::new();
        grid.set_column_spacing(10);
        grid.set_row_spacing(10);

        let image_paths = list_png_files(category.image_dir());
        let item_map = load_items(category);

        if image_paths.is_empty() {
            let empty = Label::new(Some("未找到图片资源"));
            empty.set_halign(Align::Start);
            scroll.set_child(Some(&empty));
        } else {
            for (idx, path) in image_paths.iter().enumerate() {
                let cell = build_item_cell(
                    path,
                    &item_map,
                    stats_service.clone(),
                    on_after_use.clone(),
                    &status_label,
                );
                let col = (idx % GRID_COLUMNS) as i32;
                let row = (idx / GRID_COLUMNS) as i32;
                grid.attach(&cell, col, row, 1, 1);
            }
            scroll.set_child(Some(&grid));
        }

        panel_box.append(&scroll);

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

        Self { window }
    }

    pub fn present(&self) {
        self.window.present();
    }

    pub fn toggle(&self) {
        if self.window.is_visible() {
            self.window.hide();
        } else {
            self.present();
        }
    }

    pub fn hide(&self) {
        self.window.hide();
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

    let content = Box::new(Orientation::Vertical, 4);
    content.set_halign(Align::Center);

    let image = Image::from_file(path);
    image.set_pixel_size(THUMBNAIL_SIZE);
    image.set_halign(Align::Center);
    content.append(&image);

    let filename = Path::new(path)
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
        .to_string();
    let name_label = Label::new(Some(&filename));
    name_label.set_halign(Align::Center);
    name_label.set_wrap(true);
    name_label.set_max_width_chars(12);
    content.append(&name_label);

    button.set_child(Some(&content));

    if let Some(item_def) = item_map.get(&filename).cloned() {
        let status_label = status_label.clone();
        button.connect_clicked(move |_| {
            let mut service = stats_service.clone();
            service.on_use_item(&item_def);
            status_label.set_text(&format!("已使用：{}", item_def.id));
            on_after_use();
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
