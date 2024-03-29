#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use clipboard_win::{formats, set_clipboard, Clipboard, Getter};
use warp::Filter;
use winrt_notification::{Duration, Sound, Toast, Scenario};
use platform_dirs::AppDirs;
use std::{fs, time::SystemTime,thread, path::PathBuf};
use config::Config;
use tauri::{
    CustomMenuItem, GlobalShortcutManager, Manager, SystemTray, SystemTrayEvent,
    SystemTrayMenu, SystemTrayMenuItem,
};
use tesseract;
use tpsi;
use base64::decode;
#[derive(Clone, serde::Serialize)]
struct Payload {
    message: String,
}

fn main() {
        // 启动tauri
        thread::spawn(|| {
            tauri_main();
        });
        // 启动web 进程
        let settings = Config::builder()
        .add_source(config::File::with_name("config/app.json")).build().unwrap();
        let port = settings.get_int("web").unwrap() as u16;
        if port>0 {
            wrap_web(port);
        }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn wrap_web(port : u16) {
    println!("web port: {}", port);
    // GET /hello/warp => 200 OK with body "Hello, warp!"
    let ocr = warp::path!("ocr" / String)
        .map(|name|{
            let decode = String::from_utf8(decode(&name).unwrap()).unwrap();
            // println!("ocr, {}!, base64 decode:{}",&name, decode);
            let p = PathBuf::from(decode);
            do_ocr(&p)
        }
        );
    let (_uri,srv) = warp::serve(ocr).bind_ephemeral(([0, 0, 0, 0], port));
    let handle = tokio::spawn(srv);
    handle.await.unwrap();
}
fn tauri_main(){
    let quit = CustomMenuItem::new("quit".to_string(), "退出");
    let show = CustomMenuItem::new("show".to_string(), "显示");
    let hide = CustomMenuItem::new("hide".to_string(), "隐藏");
    let tray_menu = SystemTrayMenu::new()
        .add_item(show)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(hide)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit);
    let tray = SystemTray::new().with_menu(tray_menu);

    tauri::Builder::default()
        .plugin(tpsi::init(|app, argv, cwd| {
            println!("{}, {argv:?}, {cwd}", app.package_info().name);
        }))
        .system_tray(tray)
        .on_system_tray_event(|app, event| match event {
            SystemTrayEvent::DoubleClick { position, size, .. } => {
                println!("position:{:?},size:{:?}", position, size);
                let window = app.get_window("main").unwrap();
                if window.is_visible().unwrap() {
                    window.hide().unwrap();
                } else {
                    window.set_always_on_top(true).unwrap();
                    window.show().unwrap();
                }
            }
            SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                "quit" => {
                    std::process::exit(0);
                }
                "hide" => {
                    let window = app.get_window("main").unwrap();
                    window.hide().unwrap();
                }
                "show" => {
                    let window = app.get_window("main").unwrap();
                    window.set_always_on_top(true).unwrap();
                    window.show().unwrap();
                }
                _ => {}
            },
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![img, ocr, shortcut])
        .any_thread().run(tauri::generate_context!())
        .expect("error while running tauri application");
}
#[tauri::command]
fn img() -> String {
    // 读取剪贴板内图片,没有返回空
    let _clip = Clipboard::new_attempts(10).expect("Open clipboard");
    let mut image = Vec::new();
    let clip_img = formats::Bitmap.read_clipboard(&mut image);
    let mut result = "".to_string();
    match clip_img {
        Ok(_img) => {
          let length = clip_img.expect("Read sample");
          if length > 0 {
              let img = image::load_from_memory(&image).unwrap();
              // let image_luma = img.into_luma8();
              let app_dirs = AppDirs::new(Some("ocr_tauri"), false).unwrap();
              result = app_dirs
              .data_dir
              .join("clipboard_ocr.png")
              .to_str()
              .unwrap()
              .to_string();
              fs::create_dir_all(&app_dirs.data_dir).unwrap();
              println!("{}", result);
              img.save(&result).unwrap();
              // image_luma.save(&result).unwrap();
          }
        }
        Err(_) => result ="".to_string(),
    };
    return result;
}
fn do_ocr(p: &PathBuf) ->String {
    // 识别ocr 图片
    let mut tess = tesseract::Tesseract::new(Some("tessdata"), Some("chi_sim")).unwrap();
    tess = tess.set_image(p
                .to_str()
                .unwrap(),
        )
        .unwrap();
    tess = tess.recognize().unwrap();
    return tess.get_text().unwrap();
}
#[tauri::command]
fn ocr() -> String {
    
    let sy_time = SystemTime::now();
    let app_dirs = AppDirs::new(Some("ocr_tauri"), false).unwrap();
    let p = app_dirs.data_dir.join("clipboard_ocr.png");
    let mut contents = do_ocr(&p);
    contents = contents.replace("\n\n", "\n");
    contents = contents.replace(" ", "");
    set_clipboard(formats::Unicode, &contents).expect("To set clipboard");
    
    let settings = Config::builder()
        .add_source(config::File::with_name("config/app.json")).build().unwrap();
    let notification = settings.get_bool("notification").unwrap_or(true);
    if notification {
        let toast = Toast::new(Toast::POWERSHELL_APP_ID)
        .title("OCR成功，结果已复制可以直接粘贴文本")
        .text1(&format!("耗时：{} 毫秒", SystemTime::now().duration_since(sy_time).unwrap().as_millis()))
        .text2(&contents)
        .image(p.as_path(), "the ocr image")
        .sound(Some(Sound::SMS))
        .duration(Duration::Short)
        .scenario(Scenario::Reminder)
        .show();
        match toast {
            Ok(_ok) => {},
            Err(_e) => {println!("{}", _e);},
        }
    }
    contents.to_string()
}

#[tauri::command]
async fn shortcut(app_handle: tauri::AppHandle)->String {
    let settings = Config::builder()
        .add_source(config::File::with_name("config/app.json")).build().unwrap();
    let accelerator = settings.get_string("shortcut_key").unwrap_or("ALT + C".to_string());
    let show_window = settings.get_bool("showWindowOn").unwrap_or(true);
    let w = app_handle.get_window("main").unwrap();
    if show_window {
        let _ = &w.show().unwrap();
    } else {
        let _ = &w.hide().unwrap();
    }
    app_handle.global_shortcut_manager().register(&accelerator, move || {
        if show_window {
            w.set_always_on_top(true).unwrap();
            w.show().unwrap();
        }
      app_handle.emit_all("ocr", Payload { message: "ocr".into()}).unwrap();
    }).unwrap();
    accelerator.to_string()
}
