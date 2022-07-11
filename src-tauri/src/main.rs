#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use clipboard_win::{formats, set_clipboard, Clipboard, Getter};
use winrt_notification::{Duration, Sound, Toast};
use platform_dirs::AppDirs;
use std::{fs, time::SystemTime};
use config::Config;
use tauri::{
    CustomMenuItem, GlobalShortcutManager, Manager, SystemTray, SystemTrayEvent,
    SystemTrayMenu, SystemTrayMenuItem,
};
use tesseract;
use tpsi;
#[derive(Clone, serde::Serialize)]
struct Payload {
    message: String,
}

fn main() {
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
        .run(tauri::generate_context!())
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

#[tauri::command]
fn ocr() -> String {
    // 识别ocr 图片
    let sy_time = SystemTime::now();
    let mut tess = tesseract::Tesseract::new(Some("tessdata"), Some("chi_sim")).unwrap();
    let app_dirs = AppDirs::new(Some("ocr_tauri"), false).unwrap();
    // let png = ;
    let p = app_dirs.data_dir.join("clipboard_ocr.png");
    tess = tess.set_image(p
                .to_str()
                .unwrap(),
        )
        .unwrap();
    tess = tess.recognize().unwrap();
    let mut contents = tess.get_text().unwrap();
    println!(
        "{} /n tesseract ocr end in {} seccends !",
        contents,
        SystemTime::now().duration_since(sy_time).unwrap().as_secs()
    );
    contents = contents.replace("\n\n", "\n");
    contents = contents.replace(" ", "");
    set_clipboard(formats::Unicode, &contents).expect("To set clipboard");
    
    let settings = Config::builder()
        .add_source(config::File::with_name("config/app.json")).build().unwrap();
    let notification = settings.get_bool("notification").unwrap_or(true);
    if notification {
        let toast = Toast::new(Toast::POWERSHELL_APP_ID)
        .title("OCR成功，结果已复制可以直接粘贴文本")
        .text1(&contents)
        .image(p.as_path(), "the ocr image")
        .sound(Some(Sound::SMS))
        .duration(Duration::Short)
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
    app_handle.global_shortcut_manager().register(&accelerator, move || {
        if show_window {
            w.set_always_on_top(true).unwrap();
            w.show().unwrap();
        }
      app_handle.emit_all("ocr", Payload { message: "ocr".into()}).unwrap();
    }).unwrap();
    accelerator.to_string()
}
