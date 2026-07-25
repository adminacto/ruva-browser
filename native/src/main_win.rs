use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc;

use http::Request;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy};
use tao::window::{Fullscreen, Window, WindowBuilder};
use wry::{PageLoadEvent, Rect, WebContext, WebView, WebViewBuilder};

const CHROME_HTML: &str = include_str!("../ui/chrome.html");
const NTP_HTML: &str = include_str!("../ui/ntp.html");
const SETTINGS_HTML: &str = include_str!("../ui/settings.html");
const SPLASH_HTML: &str = include_str!("../ui/splash.html");
const HISTORY_HTML: &str = include_str!("../ui/history.html");
const CONTENT_JS: &str = include_str!("../ui/content.js");
const LOGO_PNG: &[u8] = include_bytes!("../newlogo.png");

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
const DATA_DIR: &str = ".ruva";
const HARDCODED_API_KEY: &str = "";

const TAB_H: f64 = 36.0;
const NAV_H: f64 = 44.0;
const MAX_HISTORY: usize = 500;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct IpcMsg {
    cmd: String,
    #[serde(default)] url: String,
    #[serde(default)] title: String,
    #[serde(default)] key: String,
    #[serde(default)] tab_id: Option<u64>,
    #[serde(default)] on: Option<bool>,
    #[serde(default)] search_engine: String,
    #[serde(default)] homepage: Option<String>,
    #[serde(default)] bg_color: Option<String>,
    #[serde(default)] bg_image: Option<String>,
    #[serde(default)] bg_video: Option<String>,
    #[serde(default)] new_tab_show_ntp: Option<bool>,
    #[serde(default)] show_tab_bar: Option<bool>,
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
#[serde(default)]
struct Settings {
    search_engine: String, homepage: String, new_tab_show_ntp: bool,
    bg_color: String, bg_image: String, bg_video: String,
    show_tab_bar: bool, auto_show_tab_bar: bool, block_fullscreen: bool, load_images: bool,
    restore_session: bool,
    ai_api_key: String, ai_model: String, ntp: NtpSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            search_engine: "duckduckgo".into(), homepage: "ruva://newtab".into(),
            new_tab_show_ntp: true, bg_color: "#1a1a1a".into(),
            bg_image: String::new(), bg_video: String::new(),
            show_tab_bar: true, auto_show_tab_bar: false,
            block_fullscreen: false, load_images: true,
            restore_session: true,
            ai_api_key: HARDCODED_API_KEY.into(), ai_model: "openrouter/free".into(),
            ntp: NtpSettings::default(),
        }
    }
}

impl Settings {
    fn dir() -> PathBuf {
        let home = std::env::var("LOCALAPPDATA")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".into());
        PathBuf::from(home).join(DATA_DIR)
    }
    fn config_path() -> PathBuf { Self::dir().join("settings.json") }
    fn load() -> Self {
        if let Ok(data) = std::fs::read_to_string(Self::config_path()) {
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

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct HistoryEntry { url: String, title: String, time: u64 }

fn history_path() -> PathBuf { Settings::dir().join("history.json") }
fn session_path() -> PathBuf { Settings::dir().join("session.json") }

fn load_history() -> Vec<HistoryEntry> {
    if let Ok(data) = std::fs::read_to_string(history_path()) {
        serde_json::from_str(&data).unwrap_or_default()
    } else { Vec::new() }
}

fn save_history(h: &[HistoryEntry]) {
    let _ = std::fs::create_dir_all(Settings::dir());
    if let Ok(data) = serde_json::to_string(h) {
        let _ = std::fs::write(history_path(), data);
    }
}

fn load_session() -> Vec<String> {
    if let Ok(data) = std::fs::read_to_string(session_path()) {
        serde_json::from_str(&data).unwrap_or_default()
    } else { Vec::new() }
}

fn save_session(urls: &[String]) {
    let _ = std::fs::create_dir_all(Settings::dir());
    if let Ok(data) = serde_json::to_string(urls) {
        let _ = std::fs::write(session_path(), data);
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs()).unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        out.push(TABLE[(b0 >> 2) as usize] as char);
        out.push(TABLE[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        out.push(if chunk.len() > 1 { TABLE[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { TABLE[(b2 & 0x3f) as usize] as char } else { '=' });
    }
    out
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
    if trimmed.is_empty() { return String::new(); }
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return trimmed.to_string();
    }
    if trimmed.contains('.') && !trimmed.contains(' ') {
        return format!("https://{}", trimmed);
    }
    let encoded: String = trimmed.bytes().map(|b| match b {
        b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
        b' ' => "+".to_string(),
        _ => format!("%{:02X}", b),
    }).collect();
    format!("{}{}", search_url_prefix(&settings.search_engine), encoded)
}

fn bg_inject_js(settings: &Settings) -> String {
    let mut js = String::new();
    let color = if settings.bg_color.is_empty() { "#1a1a1a" } else { settings.bg_color.as_str() };
    if !settings.bg_image.is_empty() {
        let escaped = settings.bg_image.replace('\\', "\\\\").replace('\'', "\\'");
        // Keep the solid color underneath the image so transparent regions never
        // show through as a checkerboard / white background.
        js.push_str(&format!(
            "document.body.style.background=\"{} url('{}') no-repeat center center fixed\";document.body.style.backgroundSize='cover';",
            color, escaped
        ));
    } else {
        js.push_str(&format!("document.body.style.background='{}';", color));
    }
    if !settings.bg_video.is_empty() {
        let escaped = settings.bg_video.replace('\\', "\\\\").replace('\'', "\\'");
        js.push_str(&format!("(function(){{var v=document.createElement('video');v.src='{}';v.autoplay=true;v.loop=true;v.muted=true;v.style.cssText='position:fixed;top:0;left:0;width:100%;height:100%;object-fit:cover;z-index:-1';document.body.prepend(v)}})();", escaped));
    }
    js
}

fn ntp_html(settings: &Settings) -> String {
    let bg_js = bg_inject_js(settings);
    let search_prefix = search_url_prefix(&settings.search_engine);
    let ntp_json = serde_json::to_string(&settings.ntp).unwrap_or_default();
    let inject = format!(
        "<script>window.__SEARCH_URL__='{}';window.__NTP__={};window.addEventListener('DOMContentLoaded',function(){{{}}});</script>\n<script>",
        search_prefix, ntp_json, bg_js
    );
    NTP_HTML.replacen("<script>", &inject, 1)
}

fn settings_html(settings: &Settings) -> String {
    let json = serde_json::to_string(settings).unwrap_or_default();
    SETTINGS_HTML.replace("</body>", &format!("<script>window.__SETTINGS__={};try{{applySettings(window.__SETTINGS__)}}catch(e){{}}</script></body>", json))
}

fn history_html(history: &[HistoryEntry]) -> String {
    let json = serde_json::to_string(history).unwrap_or_default();
    HISTORY_HTML.replace("__HISTORY_JSON__", &json)
}

fn splash_html() -> String {
    let data_url = format!("data:image/png;base64,{}", base64_encode(LOGO_PNG));
    SPLASH_HTML.replace("__LOGO__", &data_url)
}

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

struct TabEntry {
    id: u64,
    webview: Rc<WebView>,
    url: String,
    title: String,
    loading: bool,
    is_internal: bool,
}

struct App {
    tabs: Vec<TabEntry>,
    active: usize,
    next_id: u64,
    settings: Settings,
    history: Vec<HistoryEntry>,
    html_fullscreen: bool,
}

impl App {
    fn active_tab(&self) -> Option<&TabEntry> { self.tabs.get(self.active) }
    fn tab_index(&self, id: u64) -> Option<usize> { self.tabs.iter().position(|t| t.id == id) }

    fn chrome_height(&self) -> f64 {
        if self.settings.show_tab_bar { TAB_H + NAV_H } else { NAV_H }
    }

    fn add_history(&mut self, url: &str, title: &str) {
        if !url.starts_with("http") { return; }
        // Collapse consecutive duplicates.
        if let Some(last) = self.history.first() {
            if last.url == url { return; }
        }
        self.history.insert(0, HistoryEntry { url: url.to_string(), title: title.to_string(), time: now_secs() });
        if self.history.len() > MAX_HISTORY { self.history.truncate(MAX_HISTORY); }
        save_history(&self.history);
    }

    fn save_open_session(&self) {
        let urls: Vec<String> = self.tabs.iter()
            .filter(|t| !t.is_internal && t.url.starts_with("http"))
            .map(|t| t.url.clone()).collect();
        save_session(&urls);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Src { Chrome, Splash, Tab(u64) }

// ---------------------------------------------------------------------------
// Layout + chrome sync
// ---------------------------------------------------------------------------

fn layout(window: &Window, chrome: &WebView, app: &App) {
    let size = window.inner_size();
    let scale = window.scale_factor();
    let ch: u32 = if app.html_fullscreen { 0 } else { (app.chrome_height() * scale).round() as u32 };

    let _ = chrome.set_visible(!app.html_fullscreen);
    if !app.html_fullscreen {
        let _ = chrome.set_bounds(Rect {
            position: tao::dpi::PhysicalPosition::new(0, 0).into(),
            size: tao::dpi::PhysicalSize::new(size.width.max(1), ch.max(1)).into(),
        });
    }
    for (i, t) in app.tabs.iter().enumerate() {
        if i == app.active {
            let _ = t.webview.set_bounds(Rect {
                position: tao::dpi::PhysicalPosition::new(0, ch as i32).into(),
                size: tao::dpi::PhysicalSize::new(size.width.max(1), size.height.saturating_sub(ch).max(1)).into(),
            });
            let _ = t.webview.set_visible(true);
        } else {
            let _ = t.webview.set_visible(false);
        }
    }
}

fn update_chrome(chrome: &WebView, app: &App) {
    let tabs: Vec<serde_json::Value> = app.tabs.iter().enumerate().map(|(i, t)| {
        serde_json::json!({
            "id": t.id,
            "title": if t.title.is_empty() { "Новая вкладка".to_string() } else { t.title.clone() },
            "loading": t.loading,
            "active": i == app.active,
        })
    }).collect();
    let url = app.active_tab().map(|t| if t.is_internal { String::new() } else { t.url.clone() }).unwrap_or_default();
    let state = serde_json::json!({
        "tabs": tabs,
        "url": url,
        "showTabs": app.settings.show_tab_bar,
    });
    let _ = chrome.evaluate_script(&format!("window.__ruvaUpdate&&window.__ruvaUpdate({})", state));
}

// ---------------------------------------------------------------------------
// Tab creation
// ---------------------------------------------------------------------------

fn build_tab_webview(
    window: &Window,
    ctx: &mut WebContext,
    id: u64,
    tx: &mpsc::Sender<(Src, String)>,
    proxy: &EventLoopProxy<()>,
) -> Result<WebView, wry::Error> {
    let tx_ipc = tx.clone(); let p_ipc = proxy.clone();
    let tx_title = tx.clone(); let p_title = proxy.clone();
    let tx_load = tx.clone(); let p_load = proxy.clone();

    WebViewBuilder::with_web_context(ctx)
        .with_user_agent(USER_AGENT)
        .with_background_color((16, 16, 20, 255))
        .with_initialization_script(CONTENT_JS)
        .with_ipc_handler(move |req: Request<String>| {
            let _ = tx_ipc.send((Src::Tab(id), req.body().to_string()));
            let _ = p_ipc.send_event(());
        })
        .with_document_title_changed_handler(move |title: String| {
            let msg = serde_json::json!({"cmd": "title", "title": title}).to_string();
            let _ = tx_title.send((Src::Tab(id), msg));
            let _ = p_title.send_event(());
        })
        .with_on_page_load_handler(move |ev: PageLoadEvent, url: String| {
            let loading = matches!(ev, PageLoadEvent::Started);
            let msg = serde_json::json!({"cmd": "load_state", "on": loading, "url": url}).to_string();
            let _ = tx_load.send((Src::Tab(id), msg));
            let _ = p_load.send_event(());
        })
        .build_as_child(window)
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

pub fn main() {
    let settings = Settings::load();
    let data_dir = Settings::dir();
    let _ = std::fs::create_dir_all(&data_dir);

    let mut web_context = WebContext::new(Some(data_dir.clone()));

    let event_loop = EventLoopBuilder::<()>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let (tx, rx) = mpsc::channel::<(Src, String)>();
    let (ai_tx, ai_rx) = mpsc::channel::<(u64, String)>();

    let window = WindowBuilder::new()
        .with_title("Ruva Browser")
        .with_inner_size(tao::dpi::LogicalSize::new(1280.0, 800.0))
        .with_min_inner_size(tao::dpi::LogicalSize::new(800.0, 500.0))
        .build(&event_loop)
        .unwrap();

    // ---- chrome (tab strip + toolbar) -------------------------------------
    let chrome_tx = tx.clone();
    let chrome_proxy = proxy.clone();
    let chrome_result = WebViewBuilder::with_web_context(&mut web_context)
        .with_html(CHROME_HTML)
        .with_background_color((23, 23, 28, 255))
        .with_ipc_handler(move |req: Request<String>| {
            let _ = chrome_tx.send((Src::Chrome, req.body().to_string()));
            let _ = chrome_proxy.send_event(());
        })
        .build_as_child(&window);

    let chrome = match chrome_result {
        Ok(wv) => Rc::new(wv),
        Err(e) => {
            show_error_box(&format!(
                "Failed to start the browser engine (WebView2).\n\nError: {}\n\nPlease install the Microsoft Edge WebView2 Runtime and try again.",
                e
            ));
            return;
        }
    };

    // ---- application state -------------------------------------------------
    let mut app = App {
        tabs: Vec::new(),
        active: 0,
        next_id: 1,
        settings: settings.clone(),
        history: load_history(),
        html_fullscreen: false,
    };

    // ---- restore session / first tab ---------------------------------------
    let session = if settings.restore_session { load_session() } else { Vec::new() };
    if session.is_empty() {
        let id = app.next_id; app.next_id += 1;
        if let Ok(wv) = build_tab_webview(&window, &mut web_context, id, &tx, &proxy) {
            let _ = wv.load_html(&ntp_html(&app.settings));
            app.tabs.push(TabEntry { id, webview: Rc::new(wv), url: String::new(), title: "Новая вкладка".into(), loading: false, is_internal: true });
        }
    } else {
        for url in &session {
            let id = app.next_id; app.next_id += 1;
            if let Ok(wv) = build_tab_webview(&window, &mut web_context, id, &tx, &proxy) {
                let _ = wv.load_url(url);
                app.tabs.push(TabEntry { id, webview: Rc::new(wv), url: url.clone(), title: String::new(), loading: true, is_internal: false });
            }
        }
        if app.tabs.is_empty() {
            let id = app.next_id; app.next_id += 1;
            if let Ok(wv) = build_tab_webview(&window, &mut web_context, id, &tx, &proxy) {
                let _ = wv.load_html(&ntp_html(&app.settings));
                app.tabs.push(TabEntry { id, webview: Rc::new(wv), url: String::new(), title: "Новая вкладка".into(), loading: false, is_internal: true });
            }
        }
    }

    layout(&window, &chrome, &app);
    update_chrome(&chrome, &app);

    // ---- splash overlay (on top of everything) ------------------------------
    let splash_tx = tx.clone();
    let splash_proxy = proxy.clone();
    let splash: Option<Rc<WebView>> = {
        let size = window.inner_size();
        WebViewBuilder::with_web_context(&mut web_context)
            .with_html(&splash_html())
            .with_background_color((11, 11, 15, 255))
            .with_bounds(Rect {
                position: tao::dpi::PhysicalPosition::new(0, 0).into(),
                size: tao::dpi::PhysicalSize::new(size.width.max(1), size.height.max(1)).into(),
            })
            .with_ipc_handler(move |req: Request<String>| {
                let _ = splash_tx.send((Src::Splash, req.body().to_string()));
                let _ = splash_proxy.send_event(());
            })
            .build_as_child(&window)
            .ok()
            .map(Rc::new)
    };

    let app = Rc::new(RefCell::new(app));
    let splash = Rc::new(RefCell::new(splash));
    let window = Rc::new(window);
    let web_context = Rc::new(RefCell::new(web_context));

    // Helper closure state captured by the event loop.
    let ev_app = app.clone();
    let ev_chrome = chrome.clone();
    let ev_window = window.clone();
    let ev_ctx = web_context.clone();
    let ev_splash = splash.clone();
    let ev_tx = tx.clone();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        // ---- async AI answers ------------------------------------------------
        while let Ok((tab_id, text)) = ai_rx.try_recv() {
            let a = ev_app.borrow();
            if let Some(idx) = a.tab_index(tab_id) {
                let safe = text.replace('\\', "\\\\").replace('`', "\\`").replace('\n', "<br>").replace('\r', "").replace('\'', "\\'").replace('"', "&quot;").replace('<', "&lt;").replace('>', "&gt;");
                let js = format!("if(document.getElementById('aiResponse')){{document.getElementById('aiResponse').innerHTML='{}';document.getElementById('aiLoading').style.display='none';document.getElementById('aiResponse').style.display='block';}}", safe);
                let _ = a.tabs[idx].webview.evaluate_script(&js);
            }
        }

        // ---- IPC messages ----------------------------------------------------
        while let Ok((src, raw)) = rx.try_recv() {
            let msg: IpcMsg = match serde_json::from_str(&raw) { Ok(m) => m, Err(_) => continue };
            match src {
                Src::Splash => {
                    if msg.cmd == "splash_done" {
                        if let Some(s) = ev_splash.borrow_mut().take() {
                            let _ = s.set_visible(false);
                        }
                    }
                }
                Src::Chrome => handle_chrome_msg(&msg, &ev_app, &ev_chrome, &ev_window, &ev_ctx, &ev_tx, &proxy, &ai_tx, &data_dir),
                Src::Tab(id) => handle_tab_msg(id, &msg, &ev_app, &ev_chrome, &ev_window, &ev_ctx, &ev_tx, &proxy, &ai_tx, &data_dir),
            }
        }

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    ev_app.borrow().save_open_session();
                    *control_flow = ControlFlow::Exit;
                }
                WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                    let a = ev_app.borrow();
                    layout(&ev_window, &ev_chrome, &a);
                    if let Some(s) = ev_splash.borrow().as_ref() {
                        let size = ev_window.inner_size();
                        let _ = s.set_bounds(Rect {
                            position: tao::dpi::PhysicalPosition::new(0, 0).into(),
                            size: tao::dpi::PhysicalSize::new(size.width.max(1), size.height.max(1)).into(),
                        });
                    }
                }
                _ => {}
            },
            _ => {}
        }
    });
}

// ---------------------------------------------------------------------------
// Message handlers
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn new_tab(
    app: &Rc<RefCell<App>>, chrome: &WebView, window: &Window,
    ctx: &Rc<RefCell<WebContext>>, tx: &mpsc::Sender<(Src, String)>, proxy: &EventLoopProxy<()>,
    url: Option<&str>,
) {
    let id = { let mut a = app.borrow_mut(); let id = a.next_id; a.next_id += 1; id };
    let wv = {
        let mut c = ctx.borrow_mut();
        build_tab_webview(window, &mut c, id, tx, proxy)
    };
    let Ok(wv) = wv else { return };
    let (is_internal, tab_url) = {
        let a = app.borrow();
        match url {
            Some(u) if !u.is_empty() => { let _ = wv.load_url(u); (false, u.to_string()) }
            _ => {
                if a.settings.new_tab_show_ntp || a.settings.homepage == "ruva://newtab" || a.settings.homepage.is_empty() {
                    let _ = wv.load_html(&ntp_html(&a.settings));
                    (true, String::new())
                } else {
                    let hp = a.settings.homepage.clone();
                    let _ = wv.load_url(&hp);
                    (false, hp)
                }
            }
        }
    };
    {
        let mut a = app.borrow_mut();
        a.tabs.push(TabEntry { id, webview: Rc::new(wv), url: tab_url, title: "Новая вкладка".into(), loading: !is_internal, is_internal });
        a.active = a.tabs.len() - 1;
        a.save_open_session();
    }
    let a = app.borrow();
    layout(window, chrome, &a);
    update_chrome(chrome, &a);
}

fn close_tab(app: &Rc<RefCell<App>>, chrome: &WebView, window: &Window, ctx: &Rc<RefCell<WebContext>>, tx: &mpsc::Sender<(Src, String)>, proxy: &EventLoopProxy<()>, tab_id: u64) {
    let now_empty = {
        let mut a = app.borrow_mut();
        if let Some(idx) = a.tab_index(tab_id) {
            a.tabs.remove(idx);
            if a.active >= a.tabs.len() && !a.tabs.is_empty() { a.active = a.tabs.len() - 1; }
            a.save_open_session();
        }
        a.tabs.is_empty()
    };
    if now_empty {
        new_tab(app, chrome, window, ctx, tx, proxy, None);
    } else {
        let a = app.borrow();
        layout(window, chrome, &a);
        update_chrome(chrome, &a);
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_chrome_msg(
    msg: &IpcMsg, app: &Rc<RefCell<App>>, chrome: &Rc<WebView>, window: &Rc<Window>,
    ctx: &Rc<RefCell<WebContext>>, tx: &mpsc::Sender<(Src, String)>, proxy: &EventLoopProxy<()>,
    ai_tx: &mpsc::Sender<(u64, String)>, data_dir: &PathBuf,
) {
    match msg.cmd.as_str() {
        "new_tab" => new_tab(app, chrome, window, ctx, tx, proxy, None),
        "close_tab" => {
            if let Some(id) = msg.tab_id { close_tab(app, chrome, window, ctx, tx, proxy, id); }
        }
        "switch_tab" => {
            if let Some(id) = msg.tab_id {
                let mut a = app.borrow_mut();
                if let Some(idx) = a.tab_index(id) { a.active = idx; }
                drop(a);
                let a = app.borrow();
                layout(window, chrome, &a);
                update_chrome(chrome, &a);
            }
        }
        "navigate" => {
            let url = { let a = app.borrow(); normalize_url(&msg.url, &a.settings) };
            navigate_active(app, chrome, window, &url);
        }
        "back" => { if let Some(t) = app.borrow().active_tab() { let _ = t.webview.evaluate_script("history.back()"); } }
        "forward" => { if let Some(t) = app.borrow().active_tab() { let _ = t.webview.evaluate_script("history.forward()"); } }
        "reload" => { if let Some(t) = app.borrow().active_tab() { let _ = t.webview.evaluate_script("location.reload()"); } }
        "open_settings" => open_settings(app, chrome, window),
        "open_history" => open_history(app, chrome, window, ctx, tx, proxy),
        _ => {
            // Commands shared with content pages (settings saved from settings page etc.)
            let active_id = app.borrow().active_tab().map(|t| t.id);
            if let Some(id) = active_id {
                handle_tab_msg(id, msg, app, chrome, window, ctx, tx, proxy, ai_tx, data_dir);
            }
        }
    }
}

fn navigate_active(app: &Rc<RefCell<App>>, chrome: &WebView, window: &Window, url: &str) {
    if url.is_empty() {
        let html = { let a = app.borrow(); ntp_html(&a.settings) };
        let mut a = app.borrow_mut();
        let idx = a.active;
        if let Some(t) = a.tabs.get_mut(idx) {
            t.is_internal = true; t.url.clear(); t.title = "Новая вкладка".into(); t.loading = false;
            let _ = t.webview.load_html(&html);
        }
        drop(a);
    } else {
        let mut a = app.borrow_mut();
        let idx = a.active;
        if let Some(t) = a.tabs.get_mut(idx) {
            t.is_internal = false; t.url = url.to_string(); t.loading = true;
            let _ = t.webview.load_url(url);
        }
        a.save_open_session();
        drop(a);
    }
    let a = app.borrow();
    layout(window, chrome, &a);
    update_chrome(chrome, &a);
}

fn open_settings(app: &Rc<RefCell<App>>, chrome: &WebView, window: &Window) {
    let html = { let a = app.borrow(); settings_html(&a.settings) };
    let mut a = app.borrow_mut();
    let idx = a.active;
    if let Some(t) = a.tabs.get_mut(idx) {
        t.is_internal = true; t.url.clear(); t.title = "Настройки".into(); t.loading = false;
        let _ = t.webview.load_html(&html);
    }
    drop(a);
    let a = app.borrow();
    layout(window, chrome, &a);
    update_chrome(chrome, &a);
}

fn open_history(app: &Rc<RefCell<App>>, chrome: &WebView, window: &Window, ctx: &Rc<RefCell<WebContext>>, tx: &mpsc::Sender<(Src, String)>, proxy: &EventLoopProxy<()>) {
    let _ = (ctx, tx, proxy);
    let html = { let a = app.borrow(); history_html(&a.history) };
    let mut a = app.borrow_mut();
    let idx = a.active;
    if let Some(t) = a.tabs.get_mut(idx) {
        t.is_internal = true; t.url.clear(); t.title = "История".into(); t.loading = false;
        let _ = t.webview.load_html(&html);
    }
    drop(a);
    let a = app.borrow();
    layout(window, chrome, &a);
    update_chrome(chrome, &a);
}

#[allow(clippy::too_many_arguments)]
fn handle_tab_msg(
    id: u64, msg: &IpcMsg, app: &Rc<RefCell<App>>, chrome: &Rc<WebView>, window: &Rc<Window>,
    ctx: &Rc<RefCell<WebContext>>, tx: &mpsc::Sender<(Src, String)>, proxy: &EventLoopProxy<()>,
    ai_tx: &mpsc::Sender<(u64, String)>, data_dir: &PathBuf,
) {
    match msg.cmd.as_str() {
        "title" => {
            {
                let mut a = app.borrow_mut();
                if let Some(idx) = a.tab_index(id) {
                    if !a.tabs[idx].is_internal {
                        a.tabs[idx].title = msg.title.clone();
                        let (url, title) = (a.tabs[idx].url.clone(), msg.title.clone());
                        // Refresh the newest matching history entry with the real title.
                        if let Some(h) = a.history.iter_mut().find(|h| h.url == url) {
                            if h.title.is_empty() || h.title == h.url { h.title = title; save_history(&a.history); }
                        }
                    }
                }
            }
            update_chrome(chrome, &app.borrow());
        }
        "load_state" => {
            {
                let mut a = app.borrow_mut();
                if let Some(idx) = a.tab_index(id) {
                    let loading = msg.on.unwrap_or(false);
                    a.tabs[idx].loading = loading;
                    if !a.tabs[idx].is_internal && msg.url.starts_with("http") {
                        a.tabs[idx].url = msg.url.clone();
                        if !loading {
                            let title = a.tabs[idx].title.clone();
                            let url = msg.url.clone();
                            a.add_history(&url, &title);
                            a.save_open_session();
                        }
                    }
                }
            }
            update_chrome(chrome, &app.borrow());
        }
        "fullscreen" => {
            let on = msg.on.unwrap_or(false);
            {
                let mut a = app.borrow_mut();
                a.html_fullscreen = on;
            }
            if on {
                window.set_fullscreen(Some(Fullscreen::Borderless(None)));
            } else {
                window.set_fullscreen(None);
            }
            let a = app.borrow();
            layout(window, chrome, &a);
        }
        "key" => {
            match msg.key.as_str() {
                "new_tab" => new_tab(app, chrome, window, ctx, tx, proxy, None),
                "close_tab" => close_tab(app, chrome, window, ctx, tx, proxy, id),
                "next_tab" => {
                    {
                        let mut a = app.borrow_mut();
                        if !a.tabs.is_empty() { a.active = (a.active + 1) % a.tabs.len(); }
                    }
                    let a = app.borrow();
                    layout(window, chrome, &a);
                    update_chrome(chrome, &a);
                }
                "focus_url" => {
                    let _ = chrome.evaluate_script("window.__ruvaFocusUrl&&window.__ruvaFocusUrl()");
                    let _ = chrome.focus();
                }
                "history" => open_history(app, chrome, window, ctx, tx, proxy),
                "f11" => {
                    let is_fs = window.fullscreen().is_some();
                    window.set_fullscreen(if is_fs { None } else { Some(Fullscreen::Borderless(None)) });
                }
                _ => {}
            }
        }
        "navigate" => {
            // NTP / settings / history pages navigate their own tab.
            let url = { let a = app.borrow(); normalize_url(&msg.url, &a.settings) };
            {
                let mut a = app.borrow_mut();
                if let Some(idx) = a.tab_index(id) { a.active = idx; }
            }
            navigate_active(app, chrome, window, &url);
        }
        "open_settings" => open_settings(app, chrome, window),
        "save_settings" => {
            {
                let mut a = app.borrow_mut();
                if !msg.search_engine.is_empty() { a.settings.search_engine = msg.search_engine.clone(); }
                if let Some(ref hp) = msg.homepage { a.settings.homepage = if hp.is_empty() { "ruva://newtab".into() } else { hp.clone() }; }
                if let Some(ref c) = msg.bg_color { a.settings.bg_color = c.clone(); }
                if let Some(ref i) = msg.bg_image { a.settings.bg_image = i.clone(); }
                if let Some(ref v) = msg.bg_video { a.settings.bg_video = v.clone(); }
                if let Some(v) = msg.new_tab_show_ntp { a.settings.new_tab_show_ntp = v; }
                if let Some(v) = msg.show_tab_bar { a.settings.show_tab_bar = v; }
                if let Some(ref ntp) = msg.ntp { a.settings.ntp = ntp.clone(); }
                if let Some(ref k) = msg.ai_api_key { a.settings.ai_api_key = k.clone(); }
                if let Some(ref m) = msg.ai_model { if !m.is_empty() { a.settings.ai_model = m.clone(); } }
                a.settings.save();
            }
            // Tab bar visibility may have changed.
            let a = app.borrow();
            layout(window, chrome, &a);
            update_chrome(chrome, &a);
        }
        "ai_chat" => {
            let prompt = msg.ai_prompt.clone();
            let (api_key, model) = {
                let a = app.borrow();
                let k = if a.settings.ai_api_key.is_empty() { HARDCODED_API_KEY.to_string() } else { a.settings.ai_api_key.clone() };
                (k, a.settings.ai_model.clone())
            };
            let ai_tx = ai_tx.clone();
            let ai_proxy = proxy.clone();
            std::thread::spawn(move || {
                if api_key.is_empty() {
                    let _ = ai_tx.send((id, "\u{26a0}\u{fe0f} API ключ не задан. Откройте Настройки и добавьте ключ OpenRouter.".to_string()));
                    let _ = ai_proxy.send_event(());
                    return;
                }
                let body = serde_json::json!({ "model": model, "messages": [{"role": "user", "content": prompt}], "max_tokens": 512 });
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
                                let m = &v["choices"][0]["message"];
                                if let Some(c) = m["content"].as_str() { c.to_string() }
                                else if let Some(r) = m["reasoning"].as_str() { r.to_string() }
                                else if let Some(e) = v["error"]["message"].as_str() { format!("\u{26a0}\u{fe0f} {}", e) }
                                else { raw }
                            }
                            Err(_) => raw,
                        }
                    }
                    Ok(out) => format!("\u{26a0}\u{fe0f} {}", String::from_utf8_lossy(&out.stderr)),
                    Err(_) => "\u{26a0}\u{fe0f} Ошибка соединения".to_string(),
                };
                let _ = ai_tx.send((id, text));
                let _ = ai_proxy.send_event(());
            });
        }
        "clear_data" => {
            let profile_dir = data_dir.join("EBWebView");
            let _ = std::fs::remove_dir_all(profile_dir);
        }
        "clear_history" => {
            {
                let mut a = app.borrow_mut();
                a.history.clear();
                save_history(&a.history);
            }
            open_history(app, chrome, window, ctx, tx, proxy);
        }
        _ => {}
    }
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
