use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, CssProvider, Image, STYLE_PROVIDER_PRIORITY_APPLICATION, EventControllerMotion};
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

    // 加载并显示资源图像
    let image = match load_carousel_images(&window) {
        Ok(image_widget) => image_widget,
        Err(e) => {
            // Fatal 错误：资源缺失，程序无法继续运行
            eprintln!("致命错误：无法加载图像，程序无法启动：{}", e);
            std::process::exit(1);
        }
    };
    
    // 设置窗口子部件，透明背景自动应用
    window.set_child(Some(&image));
    
    // 为Image添加事件控制器，消费鼠标事件防止穿透
    setup_image_event_handler(&image);
    
    window.present();
}

/// 加载轮播图像集
/// 从 assets/body/Default/Happy/1 目录加载所有 PNG 文件并启动轮播动画
///
/// # Returns
/// Result<Image, String> - 成功返回轮播 GTK Image Widget，失败返回错误信息
fn load_carousel_images(window: &ApplicationWindow) -> Result<Image, String> {
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
        setup_image_input_region(&window_clone, &pixbuf);
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
            // 更新输入形状为图片边界矩形
            setup_image_input_region(&window_clone, &pixbuf);
        }

        // 返回 Continue，使定时器继续运行
        glib::ControlFlow::Continue
    });

    Ok(image)
}

/// 为Image widget设置事件处理器，消费鼠标事件防止穿透
fn setup_image_event_handler(image: &Image) {
    // 创建motion事件控制器
    let motion_controller = EventControllerMotion::new();
    
    // 连接motion-enter事件，确保正确捕获鼠标进入
    motion_controller.connect_enter(|_ctrl, _x, _y| {
        // 事件被处理，不继续传播
    });
    
    image.add_controller(motion_controller);
}

/// 设置窗口输入形状为图片矩形（整个图片区域都可以接收鼠标事件）
fn setup_image_input_region(window: &ApplicationWindow, pixbuf: &gdk_pixbuf::Pixbuf) {
    let width = pixbuf.width();
    let height = pixbuf.height();
    
    if let Some(surface) = window.surface() {
        // 创建图片大小的矩形作为输入区域
        let rect = gtk4::cairo::RectangleInt::new(0, 0, width, height);
        let region = Region::create_rectangle(&rect);
        surface.set_input_region(&region);
    }
}

