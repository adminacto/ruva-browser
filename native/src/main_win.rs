use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc;

use http::Request;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::WindowBuilder;
use wry::{WebView, WebViewBuilder, WebContext};

const TOOLBAR_JS: &str = include_str!("../ui/toolbar_inject.js");
const NTP_HTML: &str = include_str!("../ui/ntp.html");
const SETTINGS_HTML: &str = include_str!("../ui/settings.html");
const SPLASH_HTML: &str = include_str!("../ui/splash.html");
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
const DATA_DIR: &str = ".ruva";
const HARDCODED_API_KEY: &str = "";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Tab { id: String, url: String, title: String, active: bool }

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct IpcMsg {
    cmd: String,
    #[serde(default)] url: String,
    #[serde(default)] title: String,
    #[serde(default)] tab_id: String,
    #[serde(default)] search_engine: String,
    #[serde(default)] homepage: Option<String>,
    #[serde(default)] bg_color: Option<String>,
    #[serde(default)] bg_image: Option<String>,
    #[serde(default)] bg_video: Option<String>,
    #[serde(default)] new_tab_show_ntp: Option<bool>,
    #[serde(default)] show_tab_bar: Option<bool>,
    #[serde(default)] auto_show_tab_bar: Option<bool>,
    #[serde(default)] block_fullscreen: Option<bool>,
    #[serde(default)] load_images: Option<bool>,
    #[serde(default)] ntp: Option<NtpSettings>,
    #[serde(default)] ai_prompt: String,
    #[serde(default)] ai_api_key: Option<String>,
    #[serde(default)] ai_model: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct NtpSettings {
    clock_color: String, greeting_color: String, search_color: String,
    clock_format_24h: bool, show_clock: bool, show_greeting: bool,
    show_date: bool, show_ai_chat: bool, quick_links: Vec<QuickLink>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct QuickLink { name: String, url: String, icon: String }

impl Default for NtpSettings {
    fn default() -> Self {
        NtpSettings {
            clock_color: "#e5e7eb".into(), greeting_color: "#9ca3af".into(),
            search_color: "#3b3b3b".into(), clock_format_24h: true,
            show_clock: true, show_greeting: true, show_date: true, show_ai_chat: true,
            quick_links: vec![
                QuickLink { name: "YouTube".into(), url: "https://youtube.com".into(), icon: "\u{1f3ac}".into() },
                QuickLink { name: "GitHub".into(), url: "https://github.com".into(), icon: "\u{1f4bb}".into() },
                QuickLink { name: "Reddit".into(), url: "https://reddit.com".into(), icon: "\u{1f916}".into() },
                QuickLink { name: "X".into(), url: "https://x.com".into(), icon: "\u{1f426}".into() },
                QuickLink { name: "Wikipedia".into(), url: "https://wikipedia.org".into(), icon: "\u{1f4d6}".into() },
            ],
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct Settings {
    search_engine: String, homepage: String, new_tab_show_ntp: bool,
    bg_color: String, bg_image: String, bg_video: String,
    show_tab_bar: bool, auto_show_tab_bar: bool, block_fullscreen: bool, load_images: bool,
    ai_api_key: String, ai_model: String, ntp: NtpSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            search_engine: "duckduckgo".into(), homepage: "ruva://newtab".into(),
            new_tab_show_ntp: true, bg_color: "#1a1a1a".into(),
            bg_image: String::new(), bg_video: String::new(),
            show_tab_bar: true, auto_show_tab_bar: false,
            block_fullscreen: true, load_images: true,
            ai_api_key: HARDCODED_API_KEY.into(), ai_model: "openrouter/free".into(),
            ntp: NtpSettings::default(),
        }
    }
}

impl Settings {
    fn config_path() -> PathBuf {
        let home = std::env::var("LOCALAPPDATA")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".into());
        PathBuf::from(home).join(DATA_DIR).join("settings.json")
    }
    fn load() -> Self {
        let path = Self::config_path();
        if let Ok(data) = std::fs::read_to_string(&path) {
            serde_json::from_str(&data).unwrap_or_default()
        } else { Self::default() }
    }
    fn save(&self) {
        let path = Self::config_path();
        let _ = std::fs::create_dir_all(path.parent().unwrap());
        if let Ok(data) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(&path, data);
        }
    }
}

struct AppState {
    tabs: Vec<Tab>, active_idx: usize, webview: Option<Rc<WebView>>,
    settings: Settings, is_ntp: bool,
}

fn search_url_prefix(engine: &str) -> &'static str {
    match engine {
        "google" => "https://www.google.com/search?q=",
        "yandex" => "https://yandex.ru/search/?text=",
        "bing" => "https://www.bing.com/search?q=",
        "brave" => "https://search.brave.com/search?q=",
        "ecosia" => "https://www.ecosia.org/search?q=",
        _ => "https://duckduckgo.com/?q=",
    }
}

fn normalize_url(input: &str, settings: &Settings) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return trimmed.to_string();
    }
    if trimmed.contains('.') && !trimmed.contains(' ') {
        return format!("https://{}", trimmed);
    }
    let encoded: String = trimmed
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            b' ' => "+".to_string(),
            _ => format!("%{:02X}", b),
        })
        .collect();
    format!("{}{}", search_url_prefix(&settings.search_engine), encoded)
}

fn bg_inject_js(settings: &Settings) -> String {
    if settings.bg_image.is_empty() && settings.bg_video.is_empty() && settings.bg_color == "#1a1a1a" {
        return String::new();
    }
    let mut js = String::new();
    if !settings.bg_color.is_empty() && settings.bg_color != "#1a1a1a" {
        js.push_str(&format!("document.body.style.background='{}';", settings.bg_color));
    }
    if !settings.bg_image.is_empty() {
        let escaped = settings.bg_image.replace('\\', "\\\\").replace('\'', "\\'");
        js.push_str(&format!("document.body.style.background=\"url('{}') no-repeat center center fixed\";", escaped));
        js.push_str("document.body.style.backgroundSize='cover';");
    }
    if !settings.bg_video.is_empty() {
        let escaped = settings.bg_video.replace('\\', "\\\\").replace('\'', "\\'");
        js.push_str(&format!("(function(){{var v=document.createElement('video');v.src='{}';v.autoplay=true;v.loop=true;v.muted=true;v.style.cssText='position:fixed;top:0;left:0;width:100%;height:100%;object-fit:cover;z-index:-1';document.body.prepend(v)}})();", escaped));
    }
    js
}

fn load_ntp_html(state: &Rc<RefCell<AppState>>) -> String {
    let (bg_js, search_prefix, ntp_json) = {
        let s = state.borrow();
        (bg_inject_js(&s.settings), search_url_prefix(&s.settings.search_engine).to_string(), serde_json::to_string(&s.settings.ntp).unwrap_or_default())
    };
    let mut inject = format!("<script>window.__SEARCH_URL__='{}';window.__NTP__={};", search_prefix, ntp_json);
    inject.push_str(&bg_js);
    inject.push_str("</script>\n<script>");
    NTP_HTML.replace("<script>", &inject)
}

/// Wraps the toolbar injection script so it can run as an initialization
/// script on every page: waits for the DOM if it is not ready yet.
fn toolbar_init_script() -> String {
    format!(
        "(function(){{function __ruvaRun(){{try{{{}}}catch(e){{}}}}if(document.readyState==='loading'){{document.addEventListener('DOMContentLoaded',__ruvaRun);}}else{{__ruvaRun();}}}})();",
        TOOLBAR_JS
    )
}

fn inject_toolbar(wv: &WebView) {
    let _ = wv.evaluate_script(&toolbar_init_script());
}

fn sync_tabs_to_toolbar(wv: &WebView, state: &AppState) {
    let tabs_json: Vec<serde_json::Value> = state.tabs.iter().map(|t| {
        serde_json::json!({"id": t.id, "title": t.title, "url": t.url, "active": t.active})
    }).collect();
    let active_id = state.tabs.get(state.active_idx).map(|t| t.id.as_str()).unwrap_or("");
    let js = format!(
        "if(window.__ruvaUpdateTabs){{window.__ruvaUpdateTabs({},'{}');}}",
        serde_json::to_string(&tabs_json).unwrap_or_default(),
        active_id
    );
    let _ = wv.evaluate_script(&js);
}

fn sync_url_to_toolbar(wv: &WebView, state: &AppState) {
    let url = state.tabs.get(state.active_idx).map(|t| t.url.as_str()).unwrap_or("");
    let title = state.tabs.get(state.active_idx).map(|t| t.title.as_str()).unwrap_or("");
    let js = format!(
        "if(window.__ruvaUpdateUrl){{window.__ruvaUpdateUrl('{}');}}",
        url.replace('\\', "\\\\").replace('\'', "\\'")
    );
    let _ = wv.evaluate_script(&js);
}

fn navigate_to(state: &Rc<RefCell<AppState>>, url: &str) {
    {
        let mut s = state.borrow_mut();
        s.is_ntp = false;
        let idx = s.active_idx;
        if let Some(tab) = s.tabs.get_mut(idx) {
            tab.url = url.to_string();
            tab.title = url::Url::parse(url).map(|u| u.host_str().unwrap_or("").to_string()).unwrap_or_default();
        }
    }
    let wv = state.borrow().webview.as_ref().unwrap().clone();
    let _ = wv.load_url(url);
}

fn load_ntp(state: &Rc<RefCell<AppState>>) {
    {
        let mut s = state.borrow_mut();
        s.is_ntp = true;
        let idx = s.active_idx;
        if let Some(tab) = s.tabs.get_mut(idx) { tab.url.clear(); tab.title = "New Tab".into(); }
    }
    let html = load_ntp_html(state);
    let wv = state.borrow().webview.as_ref().unwrap().clone();
    let _ = wv.load_html(&html);
    inject_toolbar(&wv);
}

fn load_settings_page(state: &Rc<RefCell<AppState>>) {
    {
        let mut s = state.borrow_mut();
        s.is_ntp = false;
    }
    let settings_json = { let s = state.borrow(); serde_json::to_string(&s.settings).unwrap_or_default() };
    let inject = format!("<script>window.__SETTINGS__={};</script>", settings_json);
    let html = SETTINGS_HTML.replace("</body>", &format!("{}</body>", inject));
    let wv = state.borrow().webview.as_ref().unwrap().clone();
    let _ = wv.load_html(&html);
    inject_toolbar(&wv);
}

pub fn main() {
    let settings = Settings::load();
    let data_dir = PathBuf::from(
        std::env::var("LOCALAPPDATA")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".into())
    ).join(DATA_DIR);
    let _ = std::fs::create_dir_all(&data_dir);

    // WebView2 profile (cookies, cache, localStorage) lives in the data dir.
    let mut web_context = WebContext::new(Some(data_dir.clone()));

    let event_loop = EventLoopBuilder::<()>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let (ipc_tx, ipc_rx) = mpsc::channel::<String>();
    let (ai_tx, ai_rx) = mpsc::channel::<String>();

    let window = WindowBuilder::new()
        .with_title("Ruva Browser")
        .with_inner_size(tao::dpi::LogicalSize::new(1280.0, 800.0))
        .with_min_inner_size(tao::dpi::LogicalSize::new(800.0, 600.0))
        .build(&event_loop).unwrap();

    let state = Rc::new(RefCell::new(AppState {
        tabs: vec![Tab { id: "start".into(), url: String::new(), title: "New Tab".into(), active: true }],
        active_idx: 0, webview: None, settings: settings.clone(), is_ntp: true,
    }));

    let ipc_proxy = proxy.clone();
    let splash_with_redirect = SPLASH_HTML.replace("</body>", "<script>setTimeout(function(){window.location.href='about:blank';},3000);</script></body>");
    let webview_result = WebViewBuilder::with_web_context(&mut web_context)
        .with_html(&splash_with_redirect)
        .with_user_agent(USER_AGENT)
        .with_initialization_script(&toolbar_init_script())
        .with_ipc_handler(move |req: Request<String>| {
            let _ = ipc_tx.send(req.body().to_string());
            // Wake up the event loop so the message is handled immediately.
            let _ = ipc_proxy.send_event(());
        })
        .build(&window);

    let webview = match webview_result {
        Ok(wv) => wv,
        Err(e) => {
            show_error_box(&format!(
                "Failed to start the browser engine (WebView2).\n\nError: {}\n\nPlease install the Microsoft Edge WebView2 Runtime and try again.",
                e
            ));
            return;
        }
    };

    let webview = Rc::new(webview);
    state.borrow_mut().webview = Some(webview.clone());

    let kb_state = state.clone();
    let kb_webview = webview.clone();
    let splash_proxy = proxy.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(3));
        let _ = splash_proxy.send_event(());
    });
    // Keep the web context alive for the lifetime of the app.
    let _web_context = web_context;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        let mut splash_done = false;

        while let Ok(text) = ai_rx.try_recv() {
            let safe = text.replace('\\', "\\\\").replace('`', "\\`").replace('\n', "<br>").replace('\r', "").replace('\'', "\\'").replace('"', "&quot;").replace('<', "&lt;").replace('>', "&gt;");
            let js = format!("if(document.getElementById('aiResponse')){{document.getElementById('aiResponse').innerHTML='{}';document.getElementById('aiLoading').style.display='none';document.getElementById('aiResponse').style.display='block';}}", safe);
            let _ = kb_webview.evaluate_script(&js);
        }

        while let Ok(msg_str) = ipc_rx.try_recv() {
            if let Ok(msg) = serde_json::from_str::<IpcMsg>(&msg_str) {
                match msg.cmd.as_str() {
                    "navigate" => {
                        let url = {
                            let s = kb_state.borrow();
                            normalize_url(&msg.url, &s.settings)
                        };
                        if url.is_empty() {
                            load_ntp(&kb_state);
                        } else {
                            navigate_to(&kb_state, &url);
                        }
                        let s = kb_state.borrow();
                        sync_tabs_to_toolbar(&kb_webview, &s);
                    }
                    "new_tab" => {
                        {
                            let mut st = kb_state.borrow_mut();
                            let idx = st.active_idx;
                            st.tabs[idx].active = false;
                            let new_tab = Tab { id: uuid::Uuid::new_v4().to_string(), url: String::new(), title: "New Tab".into(), active: true };
                            st.tabs.push(new_tab);
                            st.active_idx = st.tabs.len() - 1;
                            st.is_ntp = true;
                        }
                        load_ntp(&kb_state);
                        let s = kb_state.borrow();
                        sync_tabs_to_toolbar(&kb_webview, &s);
                    }
                    "back" => {
                        let _ = kb_webview.evaluate_script("history.back()");
                    }
                    "forward" => {
                        let _ = kb_webview.evaluate_script("history.forward()");
                    }
                    "reload" => {
                        let _ = kb_webview.evaluate_script("location.reload()");
                    }
                    "set_title" => {
                        let mut st = kb_state.borrow_mut();
                        let idx = st.active_idx;
                        if let Some(tab) = st.tabs.get_mut(idx) { tab.title = msg.title.clone(); }
                        drop(st);
                        let s = kb_state.borrow();
                        sync_tabs_to_toolbar(&kb_webview, &s);
                    }
                    "save_settings" => {
                        {
                            let mut st = kb_state.borrow_mut();
                            if !msg.search_engine.is_empty() { st.settings.search_engine = msg.search_engine.clone(); }
                            if let Some(ref hp) = msg.homepage { st.settings.homepage = hp.clone(); }
                            if let Some(ref c) = msg.bg_color { st.settings.bg_color = c.clone(); }
                            if let Some(ref i) = msg.bg_image { st.settings.bg_image = i.clone(); }
                            if let Some(ref v) = msg.bg_video { st.settings.bg_video = v.clone(); }
                            if let Some(v) = msg.new_tab_show_ntp { st.settings.new_tab_show_ntp = v; }
                            if let Some(v) = msg.show_tab_bar { st.settings.show_tab_bar = v; }
                            if let Some(v) = msg.auto_show_tab_bar { st.settings.auto_show_tab_bar = v; }
                            if let Some(v) = msg.block_fullscreen { st.settings.block_fullscreen = v; }
                            if let Some(v) = msg.load_images { st.settings.load_images = v; }
                            if let Some(ref ntp) = msg.ntp { st.settings.ntp = ntp.clone(); }
                            if let Some(ref k) = msg.ai_api_key { if !k.is_empty() { st.settings.ai_api_key = k.clone(); } }
                            if let Some(ref m) = msg.ai_model { st.settings.ai_model = m.clone(); }
                            st.settings.save();
                        }
                        load_ntp(&kb_state);
                        let s = kb_state.borrow();
                        sync_tabs_to_toolbar(&kb_webview, &s);
                    }
                    "open_settings" => {
                        load_settings_page(&kb_state);
                    }
                    "ai_chat" => {
                        let prompt = msg.ai_prompt.clone();
                        let api_key = { let s = kb_state.borrow(); let k = s.settings.ai_api_key.clone(); if k.is_empty() { HARDCODED_API_KEY.to_string() } else { k } };
                        let model = kb_state.borrow().settings.ai_model.clone();
                        let ai_tx_clone = ai_tx.clone();
                        let ai_proxy = proxy.clone();
                        std::thread::spawn(move || {
                            if api_key.is_empty() {
                                let _ = ai_tx_clone.send("\u{26a0}\u{fe0f} API key not set. Open Settings and add an OpenRouter API key.".to_string());
                                let _ = ai_proxy.send_event(());
                                return;
                            }
                            let body = serde_json::json!({ "model": model, "messages": [{"role": "user", "content": prompt}], "max_tokens": 256 });
                            let body_str = serde_json::to_string(&body).unwrap_or_default();
                            let output = std::process::Command::new("curl")
                                .args(["-s", "--max-time", "30", "-X", "POST", "https://openrouter.ai/api/v1/chat/completions",
                                    "-H", &format!("Authorization: Bearer {}", api_key), "-H", "Content-Type: application/json", "-d", &body_str])
                                .output();
                            let text = match output {
                                Ok(out) if out.status.success() => {
                                    let raw = String::from_utf8_lossy(&out.stdout).to_string();
                                    match serde_json::from_str::<serde_json::Value>(&raw) {
                                        Ok(v) => {
                                            let msg2 = &v["choices"][0]["message"];
                                            if let Some(c) = msg2["content"].as_str() { c.to_string() }
                                            else if let Some(r) = msg2["reasoning"].as_str() { r.to_string() }
                                            else if let Some(e) = v["error"]["message"].as_str() { format!("\u{26a0}\u{fe0f} {}", e) }
                                            else { raw }
                                        }
                                        Err(_) => raw,
                                    }
                                }
                                Ok(out) => format!("\u{26a0}\u{fe0f} {}", String::from_utf8_lossy(&out.stderr)),
                                Err(_) => "\u{26a0}\u{fe0f} Connection error".to_string(),
                            };
                            let _ = ai_tx_clone.send(text);
                            // Wake up the event loop so the answer shows up immediately.
                            let _ = ai_proxy.send_event(());
                        });
                    }
                    "clear_data" => {
                        let profile_dir = data_dir.join("EBWebView");
                        let _ = std::fs::remove_dir_all(&profile_dir);
                    }
                    "switch_tab" => {
                        {
                            let mut st = kb_state.borrow_mut();
                            let idx = st.active_idx;
                            st.tabs[idx].active = false;
                            if let Some(pos) = st.tabs.iter().position(|t| t.id == msg.tab_id) {
                                st.active_idx = pos;
                                st.tabs[pos].active = true;
                                st.is_ntp = st.tabs[pos].url.is_empty();
                            }
                        }
                        let s = kb_state.borrow();
                        if s.is_ntp {
                            drop(s);
                            load_ntp(&kb_state);
                        } else {
                            let url = s.tabs[s.active_idx].url.clone();
                            drop(s);
                            let _ = kb_webview.load_url(&url);
                            inject_toolbar(&kb_webview);
                        }
                        let s = kb_state.borrow();
                        sync_tabs_to_toolbar(&kb_webview, &s);
                    }
                    "close_tab" => {
                        let should_load_ntp;
                        let url_to_load;
                        {
                            let mut st = kb_state.borrow_mut();
                            if let Some(pos) = st.tabs.iter().position(|t| t.id == msg.tab_id) {
                                st.tabs.remove(pos);
                                if st.tabs.is_empty() {
                                    st.tabs.push(Tab { id: uuid::Uuid::new_v4().to_string(), url: String::new(), title: "New Tab".into(), active: true });
                                    st.active_idx = 0;
                                    st.is_ntp = true;
                                    should_load_ntp = true;
                                    url_to_load = String::new();
                                } else {
                                    if st.active_idx >= st.tabs.len() { st.active_idx = st.tabs.len() - 1; }
                                    st.tabs[st.active_idx].active = true;
                                    st.is_ntp = st.tabs[st.active_idx].url.is_empty();
                                    should_load_ntp = st.is_ntp;
                                    url_to_load = st.tabs[st.active_idx].url.clone();
                                }
                            } else {
                                should_load_ntp = true;
                                url_to_load = String::new();
                            }
                        }
                        if should_load_ntp {
                            load_ntp(&kb_state);
                        } else {
                            let _ = kb_webview.load_url(&url_to_load);
                            inject_toolbar(&kb_webview);
                        }
                        let s = kb_state.borrow();
                        sync_tabs_to_toolbar(&kb_webview, &s);
                    }
                    _ => {}
                }
            }
        }

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => { *control_flow = ControlFlow::Exit; }
                _ => {}
            },
            Event::UserEvent(()) => {
                if !splash_done {
                    splash_done = true;
                    load_ntp(&kb_state);
                    let s = kb_state.borrow();
                    sync_tabs_to_toolbar(&kb_webview, &s);
                }
            }
            _ => {}
        }
    });
}

fn show_error_box(msg: &str) {
    use std::ffi::CString;
    extern "system" {
        fn MessageBoxA(h_wnd: *mut std::ffi::c_void, lp_text: *const i8, lp_caption: *const i8, u_type: u32) -> i32;
    }
    let text = CString::new(msg).unwrap_or_default();
    let caption = CString::new("Ruva Browser").unwrap_or_default();
    unsafe { MessageBoxA(std::ptr::null_mut(), text.as_ptr(), caption.as_ptr(), 0x10); }
}
