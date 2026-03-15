// ===== 模块声明 =====
mod animation;
mod config;
mod drag;
mod interaction;
mod settings;
mod stats;
mod ui;
mod window;

// ===== 外部依赖导入 =====
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, CssProvider, STYLE_PROVIDER_PRIORITY_APPLICATION};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::cell::RefCell;
use std::process::Command;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

// ===== 项目内模块导入 =====
use animation::{
    is_shutdown_animation_finished, load_carousel_images, request_shutdown_animation,
    request_animation_config_reload,
    request_hover_animation_end, request_hover_animation_start,
    request_touch_body_animation, request_touch_head_animation,
};
use config::{
    load_panel_debug_config, start_panel_config_watcher, APP_ID, CAROUSEL_INTERVAL_MS,
    DEFAULT_PIXEL_SIZE,
};
use drag::setup_long_press_drag;
use interaction::{
    setup_context_menu, setup_hover_regions, setup_image_input_region, setup_input_probe,
    setup_touch_click_regions,
};
use settings::{SettingsPanel, SettingsStore};
use stats::{PetStats, PetStatsSaveStore, PetStatsService};
use ui::food::{FeedCategory, FeedPanel};
use ui::interaction::{InteractCategory, InteractionPanel};
use ui::stats::StatsPanel;
use window::position::{apply_window_position, current_window_left_top};

// ===== 系统动作状态机（用于“关机动画完成后再执行动作”） =====
#[derive(Clone, Copy, PartialEq, Eq)]
enum PendingSystemAction {
    None,
    Quit,
    Restart,
}

fn main() {
    // ===== GTK 应用入口 =====
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);

    // ===== 运行主循环并透传退出状态 =====
    let status = app.run();
    std::process::exit(status.value());
}

/// 构建应用UI
fn build_ui(app: &Application) {
    // ===== 窗口创建与基础属性 =====
    let window = ApplicationWindow::new(app);
    window.set_title(Some("Niri Pet"));
    // 不固定窗口尺寸，让内容主导窗口大小
    window.set_default_size(1, 1);

    // ===== 透明样式（窗口与图片都透明） =====
    let css_provider = CssProvider::new();
    css_provider.load_from_data(
        "window { background-color: transparent; padding: 0; margin: 0; border: 0; }
         image { background-color: transparent; padding: 0; margin: 0; border: 0; }"
    );
    window.style_context().add_provider(&css_provider, STYLE_PROVIDER_PRIORITY_APPLICATION);

    // ===== Layer Shell 配置（作为桌面浮层宠物窗口） =====
    window.init_layer_shell();

    window.set_layer(Layer::Overlay);
    // 不保留屏幕空间
    window.set_exclusive_zone(-1);
    // 不抢占键盘焦点
    window.set_keyboard_mode(KeyboardMode::None);
    // 默认锚定到右下角
    window.set_anchor(Edge::Right, true);
    window.set_anchor(Edge::Bottom, true);

    // 默认边距（避开顶部指示区与屏幕边缘）
    window.set_margin(Edge::Top, 50);
    window.set_margin(Edge::Right, 20);
    window.set_margin(Edge::Bottom, 20);

    // ===== 设置加载与窗口位置恢复 =====
    let settings_store = Rc::new(SettingsStore::load());
    if let Some(position) = settings_store.remembered_position_if_enabled() {
        apply_window_position(&window, position);
    }

    // ===== 核心运行时状态（当前图像帧 + 宠物数值服务） =====
    let current_pixbuf: Rc<RefCell<Option<gdk_pixbuf::Pixbuf>>> = Rc::new(RefCell::new(None));
    let panel_config = load_panel_debug_config();
    let stats_save_store = Rc::new(PetStatsSaveStore::load());
    let initial_stats = stats_save_store.load_stats().unwrap_or_else(PetStats::default);
    let stats_service = PetStatsService::new(initial_stats, 15.0);
    stats_service.apply_panel_config(panel_config);

    // ===== 加载动画资源并创建主图像控件 =====
    let initial_pixel_size = (DEFAULT_PIXEL_SIZE as f64 * settings_store.scale_factor()).round().max(32.0) as i32;
    let image = match load_carousel_images(&window, current_pixbuf.clone(), stats_service.clone(), initial_pixel_size) {
        Ok(image_widget) => image_widget,
        Err(e) => {
            // 资源缺失属于不可恢复错误，直接退出
            eprintln!("致命错误：无法加载图像，程序无法启动：{}", e);
            std::process::exit(1);
        }
    };

    // ===== 宠物状态逻辑 tick（按配置间隔推进） =====
    let logic_interval_secs = stats_service.logic_interval_secs();
    let stats_interval_ms = (logic_interval_secs * 1000.0) as u64;
    let mut stats_service_for_tick = stats_service.clone();
    let stats_save_store_for_tick = stats_save_store.clone();
    glib::timeout_add_local(Duration::from_millis(stats_interval_ms), move || {
        stats_service_for_tick.on_tick(logic_interval_secs);
        if let Err(err) = stats_save_store_for_tick.save_stats(&stats_service_for_tick.get_stats()) {
            eprintln!("保存角色数值存档失败：{}", err);
        }
        glib::ControlFlow::Continue
    });
    
    // ===== 装配窗口内容与统计面板 =====
    window.set_child(Some(&image));
    let stats_panel = Rc::new(StatsPanel::new(&image, stats_service.clone()));
    let on_after_feed_use = {
        let stats_panel_for_feed = stats_panel.clone();
        let stats_save_store_for_feed = stats_save_store.clone();
        let stats_service_for_feed = stats_service.clone();
        Rc::new(move || {
            stats_panel_for_feed.refresh();
            if let Err(err) = stats_save_store_for_feed.save_stats(&stats_service_for_feed.get_stats()) {
                eprintln!("物品使用后保存角色数值存档失败：{}", err);
            }
        })
    };

    let feed_panel = Rc::new(FeedPanel::new(
        app,
        &window,
        FeedCategory::Meal,
        stats_service.clone(),
        on_after_feed_use,
    ));

    let interaction_panel = Rc::new(InteractionPanel::new(
        app,
        &window,
        InteractCategory::Study,
        stats_service.clone(),
        {
            let stats_panel_for_interact = stats_panel.clone();
            let stats_save_store_for_interact = stats_save_store.clone();
            let stats_service_for_interact = stats_service.clone();
            Rc::new(move || {
                stats_panel_for_interact.refresh();
                if let Err(err) =
                    stats_save_store_for_interact.save_stats(&stats_service_for_interact.get_stats())
                {
                    eprintln!("互动后保存角色数值存档失败：{}", err);
                }
            })
        },
    ));

    // ===== 配置热更新监听：收到变更后刷新数值与动画配置 =====
    let (config_reload_tx, config_reload_rx) = mpsc::channel::<()>();
    if let Err(err) = start_panel_config_watcher(move || {
        let _ = config_reload_tx.send(());
    }) {
        eprintln!("启动配置热更新监听失败：{}", err);
    }

    {
        let stats_service = stats_service.clone();
        let stats_panel = stats_panel.clone();
        glib::timeout_add_local(Duration::from_millis(250), move || {
            let mut should_reload = false;
            while config_reload_rx.try_recv().is_ok() {
                should_reload = true;
            }

            if should_reload {
                let panel_config = load_panel_debug_config();
                stats_service.apply_panel_config(panel_config);
                stats_panel.refresh();
                request_animation_config_reload();
            }

            glib::ControlFlow::Continue
        });
    }

    // ===== 输入诊断与交互守卫状态 =====
    // 诊断：记录窗口/图片是否收到点击事件
    setup_input_probe(&window, &image);
    // 交互期间若处于“待退出/待重启”状态，则屏蔽普通输入行为
    let pending_action = Rc::new(RefCell::new(PendingSystemAction::None));

    // ===== 长按拖拽：移动窗口并按需持久化位置 =====
    setup_long_press_drag(
        &window,
        &image,
        current_pixbuf.clone(),
        stats_service.clone(),
        {
            let settings_store = settings_store.clone();
            Rc::new(move |left, top| {
                if !settings_store.remember_position_enabled() {
                    return;
                }

                if let Err(err) = settings_store.update_position(left, top) {
                    eprintln!("保存窗口位置失败：{}", err);
                }
            })
        },
        {
            let pending_action = pending_action.clone();
            Rc::new(move || *pending_action.borrow() != PendingSystemAction::None)
        },
    );

    // ===== 点击触摸区域：触发头部/身体动画 =====
    setup_touch_click_regions(
        &image,
        current_pixbuf.clone(),
        stats_service.clone(),
        Rc::new(|| {
            request_touch_head_animation();
        }),
        Rc::new(|| {
            request_touch_body_animation();
        }),
        {
            let pending_action = pending_action.clone();
            Rc::new(move || *pending_action.borrow() != PendingSystemAction::None)
        },
    );

    // niri 环境下悬浮事件走 window 级监听更稳定。
    setup_hover_regions(
        &window,
        Rc::new(|| {
            request_hover_animation_start();
        }),
        Rc::new(|| {
            request_hover_animation_end();
        }),
        {
            let pending_action = pending_action.clone();
            Rc::new(move || *pending_action.borrow() != PendingSystemAction::None)
        },
    );

    // ===== 右键上下文菜单：统计面板、设置、重启与退出 =====
    {
        let stats_panel_for_panel_click = stats_panel.clone();
        let stats_panel_for_menu_popup = stats_panel.clone();
        let feed_panel_for_menu = feed_panel.clone();
        let feed_panel_for_hide = feed_panel.clone();
        let interaction_panel_for_menu = interaction_panel.clone();
        let interaction_panel_for_hide = interaction_panel.clone();
        let settings_panel_for_menu_popup = {
            let settings_store = settings_store.clone();
            let window_for_save = window.clone();
            Rc::new(SettingsPanel::new(
                app,
                &window,
                settings_store.snapshot(),
                Rc::new(move |remember_position, scale_factor| {
                    if let Err(err) = settings_store.update_remember_position(remember_position) {
                        eprintln!("保存设置失败：{}", err);
                        return;
                    }
                    if let Err(err) = settings_store.update_scale_factor(scale_factor) {
                        eprintln!("保存缩放设置失败：{}", err);
                    }

                    if remember_position {
                        let (left, top) = current_window_left_top(&window_for_save);
                        if let Err(err) = settings_store.update_position(left, top) {
                            eprintln!("保存窗口位置失败：{}", err);
                        }
                    }
                }),
                {
                    let image_for_preview = image.clone();
                    Rc::new(move |scale_factor: f64| {
                        let pixel_size = (DEFAULT_PIXEL_SIZE as f64 * scale_factor).round().max(32.0) as i32;
                        image_for_preview.set_pixel_size(pixel_size);
                    })
                },
            ))
        };
        let app_for_quit = app.clone();

        let request_system_action = {
            let pending_action = pending_action.clone();
            let app_for_quit = app_for_quit.clone();
            let window_for_quit = window.clone();
            let settings_store_for_quit = settings_store.clone();
            let stats_service_for_quit = stats_service.clone();
            let stats_save_store_for_quit = stats_save_store.clone();
            Rc::new(move |action: PendingSystemAction| {
                // 已有待执行动作时忽略重复请求，避免并发退出流程
                if *pending_action.borrow() != PendingSystemAction::None {
                    return;
                }

                // 先播放关机动画，动画结束后再执行真正系统动作
                *pending_action.borrow_mut() = action;
                request_shutdown_animation();

                let pending_action_for_timeout = pending_action.clone();
                let app_for_timeout = app_for_quit.clone();
                let window_for_timeout = window_for_quit.clone();
                let settings_store_for_timeout = settings_store_for_quit.clone();
                let stats_service_for_timeout = stats_service_for_quit.clone();
                let stats_save_store_for_timeout = stats_save_store_for_quit.clone();
                glib::timeout_add_local(Duration::from_millis(CAROUSEL_INTERVAL_MS), move || {
                    if !is_shutdown_animation_finished() {
                        // 动画未完成，继续等待下一个周期
                        return glib::ControlFlow::Continue;
                    }

                    // 退出前按配置保存当前位置
                    if settings_store_for_timeout.remember_position_enabled() {
                        let (left, top) = current_window_left_top(&window_for_timeout);
                        if let Err(err) = settings_store_for_timeout.update_position(left, top) {
                            eprintln!("退出前保存窗口位置失败：{}", err);
                        }
                    }

                    if let Err(err) =
                        stats_save_store_for_timeout.save_stats(&stats_service_for_timeout.get_stats())
                    {
                        eprintln!("退出前保存角色数值存档失败：{}", err);
                    }

                    let action = *pending_action_for_timeout.borrow();
                    *pending_action_for_timeout.borrow_mut() = PendingSystemAction::None;

                    // 重启：拉起新进程；退出：直接 quit
                    if action == PendingSystemAction::Restart {
                        match std::env::current_exe() {
                            Ok(exe) => {
                                if let Err(err) = Command::new(exe).spawn() {
                                    eprintln!("重启失败：{}", err);
                                }
                            }
                            Err(err) => {
                                eprintln!("重启失败：无法获取当前可执行文件路径：{}", err);
                            }
                        }
                    }

                    app_for_timeout.quit();
                    glib::ControlFlow::Break
                });
            })
        };

        let request_restart = {
            let request_system_action = request_system_action.clone();
            Rc::new(move || {
                request_system_action(PendingSystemAction::Restart);
            })
        };

        let request_quit = {
            let request_system_action = request_system_action.clone();
            Rc::new(move || {
                request_system_action(PendingSystemAction::Quit);
            })
        };

        setup_context_menu(
            &image,
            Rc::new(move |x, y| {
                stats_panel_for_panel_click.toggle_at(x, y);
            }),
            Rc::new(move |menu_label| {
                match FeedCategory::from_menu_label(menu_label) {
                    Some(category) => feed_panel_for_menu.toggle_category(category),
                    None => {}
                }
            }),
            {
                let interaction_panel_for_menu = interaction_panel_for_menu.clone();
                Rc::new(move |menu_label| {
                    if let Some(category) = InteractCategory::from_menu_label(menu_label) {
                        interaction_panel_for_menu.toggle_category(category);
                    }
                })
            },
            {
                let settings_panel_for_menu_popup = settings_panel_for_menu_popup.clone();
                Rc::new(move || {
                    settings_panel_for_menu_popup.show();
                })
            },
            {
                let settings_panel_for_menu_popup = settings_panel_for_menu_popup.clone();
                Rc::new(move || {
                    stats_panel_for_menu_popup.hide();
                    feed_panel_for_hide.hide();
                    interaction_panel_for_hide.hide();
                    settings_panel_for_menu_popup.hide();
                })
            },
            request_restart,
            request_quit,
            {
                let pending_action = pending_action.clone();
                Rc::new(move || *pending_action.borrow() != PendingSystemAction::None)
            },
        );
    }
    
    // ===== 展示窗口 =====
    window.present();

    // ===== 输入区域修复策略（idle + map 双保险） =====
    // surface 就绪后至少应用一次输入区域
    let window_for_idle = window.clone();
    let image_for_idle = image.clone();
    let pixbuf_for_idle = current_pixbuf.clone();
    glib::idle_add_local_once(move || {
        if let Some(pixbuf) = pixbuf_for_idle.borrow().as_ref() {
            setup_image_input_region(&window_for_idle, &image_for_idle, pixbuf);
        }
    });

    // 在 map 后再次应用，避免初次提交时输入区域丢失
    let image_for_map = image.clone();
    let pixbuf_for_map = current_pixbuf.clone();
    window.connect_map(move |mapped_window| {
        if let Some(pixbuf) = pixbuf_for_map.borrow().as_ref() {
            setup_image_input_region(mapped_window, &image_for_map, pixbuf);
        }
    });
}

