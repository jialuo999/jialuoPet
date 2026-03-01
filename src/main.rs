mod animation;
mod config;
mod drag;
mod input_region;
mod stats;
mod stats_panel;

use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, CssProvider, GestureClick, STYLE_PROVIDER_PRIORITY_APPLICATION};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::cell::RefCell;
use std::process::Command;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

use animation::{
    is_shutdown_animation_finished, load_carousel_images, request_shutdown_animation,
    request_animation_config_reload,
    request_touch_body_animation, request_touch_head_animation,
};
use config::{
    load_panel_debug_config, start_panel_config_watcher, APP_ID, CAROUSEL_INTERVAL_MS,
};
use drag::setup_long_press_drag;
use input_region::{
    setup_context_menu, setup_image_input_region, setup_input_probe, setup_touch_click_regions,
};
use stats::PetStatsService;
use stats_panel::StatsPanel;

#[derive(Clone, Copy, PartialEq, Eq)]
enum PendingSystemAction {
    None,
    Quit,
    Restart,
}

fn main() {
    // GTK 应用主入口
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);

    // 运行应用，接收命令行参数并返回退出状态
    // 不会 panic：app.run() 的返回值是程序退出状态
    let status = app.run();
    std::process::exit(status.value());
}

/// 构建应用UI
fn build_ui(app: &Application) {
    // 创建应用窗口，绑定到应用
    // 不会 panic：ApplicationWindow::new 是标准GTK方法
    let window = ApplicationWindow::new(app);
    window.set_title(Some("Niri Pet"));
    // 不固定窗口大小，让其根据内容自动调整
    window.set_default_size(1, 1);

    // 设置透明背景 CSS
    let css_provider = CssProvider::new();
    css_provider.load_from_data(
        "window { background-color: transparent; padding: 0; margin: 0; border: 0; }
         image { background-color: transparent; padding: 0; margin: 0; border: 0; }"
    );
    window.style_context().add_provider(&css_provider, STYLE_PROVIDER_PRIORITY_APPLICATION);

    // 启用 Layer Shell：使窗口成为Niri可管理的浮窗
    window.init_layer_shell();

    // 配置 Layer Shell 属性：必须显式设置以满足Niri要求
    window.set_layer(Layer::Overlay);
    // 不保留屏幕空间，透明覆盖在其他窗口上
    window.set_exclusive_zone(-1);
    // 不抢占键盘焦点，允许其他应用响应输入
    window.set_keyboard_mode(KeyboardMode::None);
    // 默认锚定位置：右下角
    window.set_anchor(Edge::Right, true);
    window.set_anchor(Edge::Bottom, true);

    // 避开Niri顶部工作区指示器（通常高30-40px）和边缘
    window.set_margin(Edge::Top, 50);
    window.set_margin(Edge::Right, 20);
    window.set_margin(Edge::Bottom, 20);

    let current_pixbuf: Rc<RefCell<Option<gdk_pixbuf::Pixbuf>>> = Rc::new(RefCell::new(None));
    let stats_service = PetStatsService::from_panel_config(load_panel_debug_config(), 5.0);

    // 加载并显示资源图像
    let image = match load_carousel_images(&window, current_pixbuf.clone(), stats_service.clone()) {
        Ok(image_widget) => image_widget,
        Err(e) => {
            // Fatal 错误：资源缺失，程序无法继续运行
            eprintln!("致命错误：无法加载图像，程序无法启动：{}", e);
            std::process::exit(1);
        }
    };

    let logic_interval_secs = stats_service.logic_interval_secs();
    let stats_interval_ms = (logic_interval_secs * 1000.0) as u64;
    let mut stats_service_for_tick = stats_service.clone();
    glib::timeout_add_local(Duration::from_millis(stats_interval_ms), move || {
        stats_service_for_tick.on_tick(logic_interval_secs);
        glib::ControlFlow::Continue
    });
    
    // 设置窗口子部件，透明背景自动应用
    window.set_child(Some(&image));
    let stats_panel = Rc::new(StatsPanel::new(&image, stats_service.clone()));

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

    // 诊断：记录窗口/图片是否收到点击事件
    setup_input_probe(&window, &image);
    // 长按图片不透明区域后可拖动窗口位置
    setup_long_press_drag(&window, &image, current_pixbuf.clone(), stats_service.clone());
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
    );
    // 右键弹出菜单（仅在可点击区域生效）
    {
        let stats_panel_for_panel_click = stats_panel.clone();
        let stats_panel_for_menu_popup = stats_panel.clone();
        let app_for_quit = app.clone();
        let pending_action = Rc::new(RefCell::new(PendingSystemAction::None));

        let request_system_action = {
            let pending_action = pending_action.clone();
            let app_for_quit = app_for_quit.clone();
            Rc::new(move |action: PendingSystemAction| {
                if *pending_action.borrow() != PendingSystemAction::None {
                    return;
                }

                *pending_action.borrow_mut() = action;
                request_shutdown_animation();

                let pending_action_for_timeout = pending_action.clone();
                let app_for_timeout = app_for_quit.clone();
                glib::timeout_add_local(Duration::from_millis(CAROUSEL_INTERVAL_MS), move || {
                    if !is_shutdown_animation_finished() {
                        return glib::ControlFlow::Continue;
                    }

                    let action = *pending_action_for_timeout.borrow();
                    *pending_action_for_timeout.borrow_mut() = PendingSystemAction::None;

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

        setup_context_menu(&image, Rc::new(move |x, y| {
            stats_panel_for_panel_click.toggle_at(x, y);
        }), Rc::new(move || {
            stats_panel_for_menu_popup.hide();
        }), request_restart, request_quit);
    }

    let dismiss_panel_click = GestureClick::new();
    dismiss_panel_click.set_button(1);
    {
        let stats_panel = stats_panel.clone();
        dismiss_panel_click.connect_pressed(move |_, _, _, _| {
            stats_panel.hide();
        });
    }
    image.add_controller(dismiss_panel_click);
    
    window.present();

    // 确保窗口 surface 就绪后至少应用一次输入区域
    let window_for_idle = window.clone();
    let image_for_idle = image.clone();
    let pixbuf_for_idle = current_pixbuf.clone();
    glib::idle_add_local_once(move || {
        if let Some(pixbuf) = pixbuf_for_idle.borrow().as_ref() {
            setup_image_input_region(&window_for_idle, &image_for_idle, pixbuf);
        }
    });

    // 在 map 后再次应用输入区域，避免 surface 尚未提交导致输入区域丢失
    let image_for_map = image.clone();
    let pixbuf_for_map = current_pixbuf.clone();
    window.connect_map(move |mapped_window| {
        if let Some(pixbuf) = pixbuf_for_map.borrow().as_ref() {
            setup_image_input_region(mapped_window, &image_for_map, pixbuf);
        }
    });
}

