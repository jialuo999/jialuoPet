use gtk4::prelude::*;
use gtk4::{
    Align, Application, ApplicationWindow, Box, Button, CssProvider, FlowBox, Image, Label,
    Orientation, ScrolledWindow, SelectionMode, Window, STYLE_PROVIDER_PRIORITY_APPLICATION,
};
use std::cell::{Cell, RefCell};
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
    Inventory,
}

impl FeedCategory {
    const ALL: [Self; 7] = [
        Self::Meal,
        Self::Drink,
        Self::Snack,
        Self::Gift,
        Self::Drug,
        Self::Functional,
        Self::Inventory,
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
            "背包" => Some(Self::Inventory),
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
            Self::Inventory => "背包",
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
            Self::Inventory => "背包",
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
            Self::Inventory => "assets/image/Item",
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
            Self::Inventory => &[],
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
            Self::Inventory => "Inventory",
        }
    }

    pub fn is_inventory(&self) -> bool {
        *self == Self::Inventory
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
    selected_items: Rc<RefCell<Vec<ItemDef>>>,
    buy_button: Button,
    buy_and_use_button: Button,
    action_buttons_box: Box,
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
                    min-height: {}px;\n\
                    padding: 0;\n\
                    margin: 0;\n\
                }}\n\
                .feed-sidebar-button,\n\
                .feed-sidebar-button:hover,\n\
                .feed-sidebar-button:active,\n\
                .feed-sidebar-button:checked {{\n\
                    min-width: {}px;\n\
                    padding-left: 2px;\n\
                    padding-right: 2px;\n\
                }}",
                ITEM_CELL_WIDTH,
                ITEM_CELL_HEIGHT,
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


        let action_buttons_box = Box::new(Orientation::Horizontal, 8);
        let buy_button = Button::with_label("购买");
        let buy_and_use_button = Button::with_label("购买并使用");
        action_buttons_box.append(&buy_button);
        action_buttons_box.append(&buy_and_use_button);

        let close_button = Button::with_label("退出");

        let bottom_box = Box::new(Orientation::Horizontal, 8);
        bottom_box.set_hexpand(true);
        bottom_box.set_halign(Align::End);
        bottom_box.append(&action_buttons_box);
        bottom_box.append(&close_button);
        panel_box.append(&bottom_box);

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
            selected_items: Rc::new(RefCell::new(Vec::new())),
            buy_button,
            buy_and_use_button,
            action_buttons_box,
        };

        panel.connect_sidebar_handlers();
        panel.connect_action_buttons();
        panel.switch_category(category);

        panel
    }

    pub fn present(&self) {
        self.window.present();
    }

    pub fn toggle_category(&self, category: FeedCategory) {
        self.switch_category(category);
        self.present();
    }

    pub fn switch_category(&self, category: FeedCategory) {
        self.current_category.set(category);
        self.window.set_title(Some(category.panel_title()));
        self.title.set_text(category.panel_title());
        self.selected_items.borrow_mut().clear();
        
        if category.is_inventory() {
            self.status_label.set_text("双击物品可使用");
            self.action_buttons_box.set_visible(false);
        } else {
            self.status_label.set_text("单击物品可选中、查看或购买");
            self.action_buttons_box.set_visible(true);
        }
        
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

    fn connect_action_buttons(&self) {
        let panel_buy = self.clone_ref();
        self.buy_button.connect_clicked(move |_| {
            let items = panel_buy.selected_items.borrow().clone();
            if items.is_empty() {
                panel_buy.status_label.set_text("请先选中物品");
                return;
            }
            let mut service = panel_buy.stats_service.clone();
            let mut bought = Vec::new();
            let mut failed = Vec::new();
            for item in &items {
                if service.buy_and_add_to_inventory(item) {
                    bought.push(item.id.clone());
                } else {
                    failed.push(format!("{} 需要 {}", item.id, item.price));
                }
            }
            if !failed.is_empty() {
                panel_buy.status_label.set_text(&format!("金钱不足：{}", failed.join("、")));
            } else {
                panel_buy.status_label.set_text(&format!("已购买放入背包：{}", bought.join("、")));
            }

            if panel_buy.should_auto_clear_shop_selection() {
                panel_buy.clear_shop_selection();
            }
        });

        let panel_use = self.clone_ref();
        self.buy_and_use_button.connect_clicked(move |_| {
            let items = panel_use.selected_items.borrow().clone();
            if items.is_empty() {
                panel_use.status_label.set_text("请先选中物品");
                return;
            }
            let mut service = panel_use.stats_service.clone();
            let mut results = Vec::new();
            let mut any_used = false;
            for item in &items {
                // 优先从背包使用，如果没有则直接购买使用
                if service.get_inventory_count(&item.id) > 0 {
                    if service.use_from_inventory(item) {
                        results.push(format!("{}（背包）", item.id));
                        any_used = true;
                    }
                } else {
                    if service.on_use_item(item) {
                        results.push(format!("{}（购买）", item.id));
                        any_used = true;
                    } else {
                        results.push(format!("{}×金钱不足", item.id));
                    }
                }
            }
            panel_use.status_label.set_text(&format!("已使用：{}", results.join("、")));
            if any_used {
                (panel_use.on_after_use)();
            }

            if panel_use.should_auto_clear_shop_selection() {
                panel_use.clear_shop_selection();
            }
        });
    }

    fn should_auto_clear_shop_selection(&self) -> bool {
        !self.current_category.get().is_inventory()
    }

    fn clear_shop_selection(&self) {
        // 重新加载格子以重置每个单元格内部维护的选中状态和高亮样式。
        self.selected_items.borrow_mut().clear();
        self.reload_items_for(self.current_category.get());
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

        if category.is_inventory() {
            self.load_inventory_items();
        } else {
            self.load_shop_items(category);
        }
    }

    fn load_shop_items(&self, category: FeedCategory) {
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
                self.selected_items.clone(),
                &self.status_label,
            );
            self.flow.insert(&cell, -1);
        }
    }

    fn load_inventory_items(&self) {
        let inventory = self.stats_service.list_inventory();
        let all_items = load_all_items();

        if inventory.is_empty() {
            let empty = Label::new(Some("背包为空"));
            empty.set_halign(Align::Center);
            self.flow.insert(&empty, -1);
            return;
        }

        // 热加载回调：用完物品后立即刷新背包界面
        let panel_for_refresh = self.clone_ref();
        let on_refresh: Rc<dyn Fn()> = Rc::new(move || {
            panel_for_refresh.reload_items_for(FeedCategory::Inventory);
        });

        for (item_id, count) in inventory {
            if let Some(item_def) = all_items.get(&item_id) {
                let cell = build_inventory_item_cell(
                    item_def,
                    count,
                    self.stats_service.clone(),
                    self.on_after_use.clone(),
                    on_refresh.clone(),
                    &self.status_label,
                );
                self.flow.insert(&cell, -1);
            }
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
            selected_items: self.selected_items.clone(),
            buy_button: self.buy_button.clone(),
            buy_and_use_button: self.buy_and_use_button.clone(),
            action_buttons_box: self.action_buttons_box.clone(),
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
    _stats_service: PetStatsService,
    selected_items: Rc<RefCell<Vec<ItemDef>>>,
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
        let selected_items_cb = selected_items.clone();
        let is_selected = Rc::new(Cell::new(false));
        
        button.connect_clicked(move |btn| {
            {
                let mut selected = selected_items_cb.borrow_mut();
                if is_selected.get() {
                    // 再次点击 → 取消选中
                    is_selected.set(false);
                    btn.remove_css_class("suggested-action");
                    selected.retain(|i| i.id != item_def.id);
                } else {
                    // 首次点击 → 选中（多选）
                    is_selected.set(true);
                    btn.add_css_class("suggested-action");
                    selected.push(item_def.clone());
                }
            }
            // 更新状态栏
            let selected = selected_items_cb.borrow();
            if selected.is_empty() {
                status_label.set_text("单击物品可选中、查看或购买");
            } else if selected.len() == 1 {
                status_label.set_text(&format!(
                    "已选中：{} - 价格: {}",
                    selected[0].id, selected[0].price
                ));
            } else {
                let total: u32 = selected.iter().map(|i| i.price).sum();
                status_label.set_text(&format!(
                    "已选中 {} 件 - 总价: {}",
                    selected.len(), total
                ));
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

fn build_inventory_item_cell(
    item_def: &ItemDef,
    count: u32,
    stats_service: PetStatsService,
    on_after_use: Rc<dyn Fn()>,
    on_refresh: Rc<dyn Fn()>,
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

    // 根据物品 kind 确定所属分类图片目录，加载图片
    let image_dir = image_dir_for_kind(item_def.kind);
    let image_path = format!("{}/{}.png", image_dir, item_def.id);
    if Path::new(&image_path).exists() {
        let image = Image::from_file(&image_path);
        image.set_pixel_size(THUMBNAIL_SIZE);
        image.set_halign(Align::Center);
        content.append(&image);
    } else {
        let placeholder = Label::new(Some("?"));
        placeholder.set_size_request(THUMBNAIL_SIZE, THUMBNAIL_SIZE);
        placeholder.set_halign(Align::Center);
        placeholder.set_valign(Align::Center);
        content.append(&placeholder);
    }

    // 显示物品名称和数量
    let name_viewport = Box::new(Orientation::Horizontal, 0);
    name_viewport.set_width_request(ITEM_NAME_VIEWPORT_WIDTH);
    name_viewport.set_height_request(ITEM_NAME_VIEWPORT_HEIGHT);
    name_viewport.set_halign(Align::Center);
    name_viewport.set_valign(Align::Center);
    let name_label = Label::new(None);
    name_label.set_halign(Align::Center);
    name_label.set_valign(Align::Center);
    name_label.set_wrap(false);
    name_label.set_single_line_mode(true);
    name_label.set_tooltip_text(Some(&item_def.id));
    {
        let font_size = ITEM_NAME_FONT_PT * gtk4::pango::SCALE;
        let escaped = glib::markup_escape_text(&item_def.id);
        name_label.set_markup(&format!(
            "<span size=\"{}\">{} x{}</span>",
            font_size, escaped, count
        ));
    }
    name_viewport.append(&name_label);
    content.append(&name_viewport);

    button.set_child(Some(&content));

    let item_def_click = item_def.clone();
    let status_label = status_label.clone();
    let click_count = Rc::new(Cell::new(0));
    let click_time = Rc::new(Cell::new(std::time::Instant::now()));

    button.connect_clicked({
        let click_count = click_count.clone();
        let click_time = click_time.clone();
        let item_for_click = item_def_click.clone();
        
        move |_| {
            let now = std::time::Instant::now();
            let elapsed = now.duration_since(click_time.get()).as_millis();
            
            if elapsed < 300 {
                click_count.set(click_count.get() + 1);
                if click_count.get() >= 2 {
                    // 双击：使用物品
                    let mut service = stats_service.clone();
                    if service.use_from_inventory(&item_for_click) {
                        status_label.set_text(&format!("已使用：{}", item_for_click.id));
                        on_after_use();
                        // 热加载：用完后立即刷新背包界面（数量减少或消失）
                        on_refresh();
                    } else {
                        status_label.set_text(&format!("背包中已无：{}", item_for_click.id));
                    }
                    click_count.set(0);
                }
            } else {
                click_count.set(1);
            }
            click_time.set(now);
        }
    });

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

fn load_all_items() -> HashMap<String, ItemDef> {
    let mut all_items = HashMap::new();
    
    // 加载所有非库存分类的物品
    for category in FeedCategory::all() {
        if !category.is_inventory() {
            let items = load_items(*category);
            all_items.extend(items);
        }
    }
    
    all_items
}

fn image_dir_for_kind(kind: ItemKind) -> &'static str {
    match kind {
        ItemKind::Staple => "assets/image/food/meal",
        ItemKind::Drink => "assets/image/food/drink",
        ItemKind::Snack => "assets/image/food/snack",
        ItemKind::Gift => "assets/image/food/gift",
        ItemKind::Drug => "assets/image/food/drug",
        ItemKind::Functional => "assets/image/food/functional",
    }
}
