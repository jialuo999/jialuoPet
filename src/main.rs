use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, CssProvider, GestureClick, Image, STYLE_PROVIDER_PRIORITY_APPLICATION};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use gtk4::cairo::Region;
use std::path::PathBuf;
use std::fs;
use glib::timeout_add_local;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

const APP_ID: &str = "com.jialuo.niripet";
const CAROUSEL_INTERVAL_MS: u64 = 150; // 150ms 轮播间隔
const INPUT_DEBUG_LOG: bool = false;

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

    // 加载并显示资源图像
    let image = match load_carousel_images(&window, current_pixbuf.clone()) {
        Ok(image_widget) => image_widget,
        Err(e) => {
            // Fatal 错误：资源缺失，程序无法继续运行
            eprintln!("致命错误：无法加载图像，程序无法启动：{}", e);
            std::process::exit(1);
        }
    };
    
    // 设置窗口子部件，透明背景自动应用
    window.set_child(Some(&image));

    // 诊断：记录窗口/图片是否收到点击事件
    setup_input_probe(&window, &image);
    
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

/// 加载轮播图像集
/// 从 assets/body/Default/Happy/1 目录加载所有 PNG 文件并启动轮播动画
///
/// # Returns
/// Result<Image, String> - 成功返回轮播 GTK Image Widget，失败返回错误信息
fn load_carousel_images(
    window: &ApplicationWindow,
    current_pixbuf: Rc<RefCell<Option<gdk_pixbuf::Pixbuf>>>,
) -> Result<Image, String> {
    let asset_dir = PathBuf::from("/home/jialuo/Code/jialuoPet/assets/body/Default/Happy/1");

    if !asset_dir.is_dir() {
        return Err(format!("目录不存在：{}", asset_dir.display()));
    }

    // 读取目录中的所有 PNG 文件并排序
    let mut image_files: Vec<PathBuf> = fs::read_dir(&asset_dir)
        .map_err(|e| format!("无法读取目录 {}: {}", asset_dir.display(), e))?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("png") {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    if image_files.is_empty() {
        return Err(format!("目录中没有找到 PNG 文件：{}", asset_dir.display()));
    }

    // 排序文件名，确保播放顺序正确
    image_files.sort();

    // 创建 Image Widget
    let image = Image::new();
    image.set_pixel_size(256);

    // 初始化状态：Rc<RefCell<>> 用于在闭包中共享可变状态
    let state = Rc::new(RefCell::new((0usize, image_files.clone())));
    let state_clone = state.clone();
    let image_clone = image.clone();
    let window_clone = window.clone();

    // 初始化第一张图片
    if let Ok(pixbuf) = gdk_pixbuf::Pixbuf::from_file(&image_files[0]) {
        image_clone.set_from_pixbuf(Some(&pixbuf));
        *current_pixbuf.borrow_mut() = Some(pixbuf.clone());
        setup_image_input_region(&window_clone, &image_clone, &pixbuf);
    }

    // 设置定时器，每 150ms 更新一次图片
    timeout_add_local(Duration::from_millis(CAROUSEL_INTERVAL_MS), move || {
        let (next_path, _) = {
            let mut state_mut = state_clone.borrow_mut();
            let current_index = state_mut.0;
            let image_paths = &state_mut.1;

            // 更新到下一张图片
            let next_index = (current_index + 1) % image_paths.len();
            let next_path = image_paths[next_index].clone();
            
            // 更新状态
            state_mut.0 = next_index;

            (next_path, true)
        };

        // 加载并显示下一张图片
        if let Ok(pixbuf) = gdk_pixbuf::Pixbuf::from_file(&next_path) {
            image_clone.set_from_pixbuf(Some(&pixbuf));
            *current_pixbuf.borrow_mut() = Some(pixbuf.clone());
            // 更新输入形状为图片不透明区域
            setup_image_input_region(&window_clone, &image_clone, &pixbuf);
        }

        // 返回 Continue，使定时器继续运行
        glib::ControlFlow::Continue
    });

    Ok(image)
}

/// 设置窗口输入形状：仅图片不透明区域接收鼠标事件
fn setup_image_input_region(
    window: &ApplicationWindow,
    image: &Image,
    pixbuf: &gdk_pixbuf::Pixbuf,
) {
    let Some(surface) = window.surface() else {
        eprintln!("[input-region] skipped: window surface is None");
        return;
    };

    let alloc = image.allocation();
    let (offset_x, offset_y, render_w, render_h) = (alloc.x(), alloc.y(), alloc.width(), alloc.height());

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

fn setup_input_probe(window: &ApplicationWindow, image: &Image) {
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

/// 根据 pixbuf 的 Alpha 通道创建输入区域（缩放到 widget 实际渲染坐标）
/// Alpha > 0 的像素为可点击区域，透明背景会穿透
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
                    let rect = gtk4::cairo::RectangleInt::new(offset_x + start, offset_y + dy, dx - start, 1);
                    let _ = region.union_rectangle(&rect);
                    run_start = None;
                }
                _ => {}
            }
        }

        if let Some(start) = run_start {
            let rect = gtk4::cairo::RectangleInt::new(
                offset_x + start,
                offset_y + dy,
                render_w - start,
                1,
            );
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

