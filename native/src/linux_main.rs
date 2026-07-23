use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::mpsc;

use gtk::prelude::*;
use http::Request;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::platform::unix::WindowExtUnix;
use tao::window::WindowBuilder;
use wry::{WebView, WebViewBuilder, WebViewBuilderExtUnix, WebContext};

const NTP_HTML: &str = include_str!("../ui/ntp.html");
const SETTINGS_HTML: &str = include_str!("../ui/settings.html");
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
const DATA_DIR: &str = ".ruva";
const HARDCODED_API_KEY: &str = "";

const FULLSCREEN_BLOCK_JS: &str = r#"
(function(){
  var noop = function(){};
  try {
    if(Document.prototype.requestFullscreen) Document.prototype.requestFullscreen = noop;
    if(Document.prototype.webkitRequestFullscreen) Document.prototype.webkitRequestFullscreen = noop;
    if(Element.prototype.requestFullscreen) Element.prototype.requestFullscreen = noop;
    if(Element.prototype.webkitRequestFullscreen) Element.prototype.webkitRequestFullscreen = noop;
  } catch(e){}
})();
"#;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Tab {
    id: String,
    url: String,
    title: String,
    active: bool,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct IpcMsg {
    cmd: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    search_engine: String,
    #[serde(default)]
    homepage: Option<String>,
    #[serde(default)]
    bg_color: Option<String>,
    #[serde(default)]
    bg_image: Option<String>,
    #[serde(default)]
    bg_video: Option<String>,
    #[serde(default)]
    new_tab_show_ntp: Option<bool>,
    #[serde(default)]
    show_tab_bar: Option<bool>,
    #[serde(default)]
    auto_show_tab_bar: Option<bool>,
    #[serde(default)]
    block_fullscreen: Option<bool>,
    #[serde(default)]
    load_images: Option<bool>,
    #[serde(default)]
    ntp: Option<NtpSettings>,
    #[serde(default)]
    ai_prompt: String,
    #[serde(default)]
    ai_api_key: Option<String>,
    #[serde(default)]
    ai_model: Option<String>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct Settings {
    search_engine: String,
    homepage: String,
    new_tab_show_ntp: bool,
    bg_color: String,
    bg_image: String,
    bg_video: String,
    show_tab_bar: bool,
    auto_show_tab_bar: bool,
    block_fullscreen: bool,
    load_images: bool,
    #[serde(default)]
    ai_api_key: String,
    #[serde(default = "default_ai_model")]
    ai_model: String,
    #[serde(default)]
    ntp: NtpSettings,
}

impl Settings {
    fn config_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        PathBuf::from(home).join(DATA_DIR).join("settings.json")
    }

    fn load() -> Self {
        let path = Self::config_path();
        if let Ok(data) = std::fs::read_to_string(&path) {
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            Self::default()
        }
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
struct NtpSettings {
    clock_color: String,
    greeting_color: String,
    search_color: String,
    clock_format_24h: bool,
    show_clock: bool,
    show_greeting: bool,
    show_date: bool,
    show_ai_chat: bool,
    quick_links: Vec<QuickLink>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct QuickLink {
    name: String,
    url: String,
    icon: String,
}

impl Default for NtpSettings {
    fn default() -> Self {
        NtpSettings {
            clock_color: "#e5e7eb".into(),
            greeting_color: "#9ca3af".into(),
            search_color: "#3b3b3b".into(),
            clock_format_24h: true,
            show_clock: true,
            show_greeting: true,
            show_date: true,
            show_ai_chat: true,
            quick_links: vec![
                QuickLink { name: "YouTube".into(), url: "https://youtube.com".into(), icon: "\u{1f3ac}".into() },
                QuickLink { name: "GitHub".into(), url: "https://github.com".into(), icon: "\u{1f4bb}".into() },
                QuickLink { name: "Reddit".into(), url: "https://reddit.com".into(), icon: "\u{1f916}".into() },
                QuickLink { name: "X".into(), url: "https://x.com".into(), icon: "\u{1f426}".into() },
                QuickLink { name: "Wikipedia".into(), url: "https://wikipedia.org".into(), icon: "\u{1f4d6}".into() },
                QuickLink { name: "\u{041f}\u{0435}\u{0440}\u{0435}\u{0432}\u{043e}\u{0434}\u{0447}\u{0438}\u{043a}".into(), url: "https://translate.google.com".into(), icon: "\u{1f310}".into() },
                QuickLink { name: "ChatGPT".into(), url: "https://chatgpt.com".into(), icon: "\u{1f916}".into() },
                QuickLink { name: "DuckDuckGo".into(), url: "https://duckduckgo.com".into(), icon: "\u{1f984}".into() },
            ],
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            search_engine: "duckduckgo".into(),
            homepage: "ruva://newtab".into(),
            new_tab_show_ntp: true,
            bg_color: "#1a1a1a".into(),
            bg_image: String::new(),
            bg_video: String::new(),
            show_tab_bar: true,
            auto_show_tab_bar: false,
            block_fullscreen: true,
            load_images: true,
            ai_api_key: HARDCODED_API_KEY.into(),
            ai_model: default_ai_model(),
            ntp: NtpSettings::default(),
        }
    }
}

fn default_ai_model() -> String {
    "openrouter/free".to_string()
}

struct AppState {
    tabs: Vec<Tab>,
    active_idx: usize,
    webview: Option<Rc<WebView>>,
    tab_revealer: Option<gtk::Revealer>,
    url_entry: Option<gtk::Entry>,
    settings: Settings,
    on_settings_page: bool,
}

impl AppState {
    fn search_url(&self, query: &str) -> String {
        match self.settings.search_engine.as_str() {
            "google" => format!("https://www.google.com/search?q={}", query),
            "yandex" => format!("https://yandex.ru/search/?text={}", query),
            "bing" => format!("https://www.bing.com/search?q={}", query),
            "brave" => format!("https://search.brave.com/search?q={}", query),
            "ecosia" => format!("https://www.ecosia.org/search?q={}", query),
            _ => format!("https://duckduckgo.com/?q={}", query),
        }
    }
}

fn normalize_url(state: &AppState, val: &str) -> String {
    let trimmed = val.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        trimmed.to_string()
    } else if trimmed.contains('.') && !trimmed.contains(' ') {
        format!("https://{}", trimmed)
    } else {
        state.search_url(&trimmed.replace(' ', "+"))
    }
}

fn get_wv(state: &Rc<RefCell<AppState>>) -> Rc<WebView> {
    state.borrow().webview.as_ref().unwrap().clone()
}

fn get_entry(state: &Rc<RefCell<AppState>>) -> gtk::Entry {
    state.borrow().url_entry.as_ref().unwrap().clone()
}

fn get_tab_revealer(state: &Rc<RefCell<AppState>>) -> gtk::Revealer {
    state.borrow().tab_revealer.as_ref().unwrap().clone()
}

fn refresh_tab_bar(state: &Rc<RefCell<AppState>>) {
    let tab_revealer = get_tab_revealer(state);
    let url_entry = get_entry(state);

    let tab_box = tab_revealer.children().first()
        .and_then(|w| w.clone().downcast::<gtk::ScrolledWindow>().ok())
        .and_then(|sw| sw.children().first().cloned())
        .and_then(|c| c.downcast::<gtk::Box>().ok());
    if let Some(ref tb) = tab_box {
        while let Some(child) = tb.children().first() {
            tb.remove(child);
        }
    }
    let tab_box = tab_box.unwrap_or_else(|| gtk::Box::new(gtk::Orientation::Horizontal, 2));

    let tabs_data: Vec<(String, String, bool)> = {
        let s = state.borrow();
        s.tabs.iter().enumerate()
            .map(|(i, t)| (t.id.clone(), t.title.clone(), i == s.active_idx))
            .collect()
    };

    let current_url = {
        let s = state.borrow();
        s.tabs.get(s.active_idx).map(|t| t.url.clone()).unwrap_or_default()
    };
    url_entry.set_text(&current_url);

    for (i, (id, title, is_active)) in tabs_data.iter().enumerate() {
        let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 4);

        let display_title = if title.len() > 20 {
            format!("{}...", &title[..17])
        } else {
            title.clone()
        };
        let label = gtk::Label::new(Some(&display_title));
        label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        label.set_max_width_chars(20);
        hbox.pack_start(&label, true, true, 0);

        let close_btn = gtk::Button::with_label("\u{00d7}");
        close_btn.set_size_request(18, 18);
        close_btn.set_relief(gtk::ReliefStyle::None);
        close_btn.style_context().add_class("tab-close");
        {
            let s = state.clone();
            let close_id = id.clone();
            close_btn.connect_clicked(move |_| {
                let idx = {
                    let st = s.borrow();
                    st.tabs.iter().position(|t| t.id == close_id).unwrap_or(0)
                };
                close_tab_at(&s, idx);
            });
        }
        hbox.pack_start(&close_btn, false, false, 0);

        let btn = gtk::Button::new();
        btn.set_relief(gtk::ReliefStyle::None);
        btn.style_context().add_class("tab-btn");
        if *is_active {
            btn.style_context().add_class("active-tab");
        }
        btn.add(&hbox);

        {
            let s = state.clone();
            let tab_idx = i;
            btn.connect_clicked(move |_| {
                let url = {
                    let mut st = s.borrow_mut();
                    if tab_idx < st.tabs.len() {
                        st.active_idx = tab_idx;
                        st.tabs[tab_idx].url.clone()
                    } else {
                        return;
                    }
                };
                let wv = get_wv(&s);
                if url.is_empty() {
                    let _ = wv.load_html(NTP_HTML);
                } else {
                    let _ = wv.load_url(&url);
                }
                refresh_tab_bar(&s);
            });
        }

        tab_box.pack_start(&btn, false, false, 0);
    }
    tab_box.show_all();
}

fn navigate_to(state: &Rc<RefCell<AppState>>, url: &str) {
    {
        let mut s = state.borrow_mut();
        s.on_settings_page = false;
        let idx = s.active_idx;
        if let Some(tab) = s.tabs.get_mut(idx) {
            tab.url = url.to_string();
            tab.title = url::Url::parse(url)
                .map(|u| u.host_str().unwrap_or("").to_string())
                .unwrap_or_default();
        }
    }
    let wv = get_wv(state);
    let _ = wv.load_url(url);
    refresh_tab_bar(state);
}

fn search_url_prefix(engine: &str) -> &str {
    match engine {
        "google" => "https://www.google.com/search?q=",
        "yandex" => "https://yandex.ru/search/?text=",
        "bing" => "https://www.bing.com/search?q=",
        "brave" => "https://search.brave.com/search?q=",
        "ecosia" => "https://www.ecosia.org/search?q=",
        _ => "https://duckduckgo.com/?q=",
    }
}

fn bg_inject_js(settings: &Settings) -> String {
    let mut js = String::new();
    if !settings.bg_color.is_empty() && settings.bg_color != "#1a1a1a" {
        js.push_str(&format!("document.body.style.background='{}';", settings.bg_color));
    }
    if !settings.bg_image.is_empty() {
        let escaped = settings.bg_image.replace('\\', "\\\\").replace('\'', "\\'");
        js.push_str(&format!(
            "document.body.style.background='url(\\'{}\\') center/cover no-repeat';",
            escaped
        ));
    }
    if !settings.bg_video.is_empty() {
        let escaped = settings.bg_video.replace('\\', "\\\\").replace('\'', "\\'");
        js.push_str(&format!(
            "(function(){{var v=document.createElement('video');v.src='{}';v.autoplay=true;v.loop=true;v.muted=true;v.style.cssText='position:fixed;top:0;left:0;width:100vw;height:100vh;object-fit:cover;z-index:-1;pointer-events:none;';document.body.prepend(v);}})();",
            escaped
        ));
    }
    js
}

fn load_ntp(state: &Rc<RefCell<AppState>>) {
    {
        let mut s = state.borrow_mut();
        s.on_settings_page = false;
        let idx = s.active_idx;
        if let Some(tab) = s.tabs.get_mut(idx) {
            tab.url.clear();
            tab.title = "New Tab".into();
        }
    }
    let wv = get_wv(state);
    let html = load_ntp_html(state);
    let _ = wv.load_html(&html);
    refresh_tab_bar(state);
}

fn load_settings_page(state: &Rc<RefCell<AppState>>) {
    {
        let mut s = state.borrow_mut();
        s.on_settings_page = true;
    }
    let settings_json = {
        let s = state.borrow();
        serde_json::to_string(&s.settings).unwrap_or_default()
    };
    let wv = get_wv(state);
    let inject = format!(
        "<script>window.__SETTINGS__={};</script>",
        settings_json
    );
    let html = SETTINGS_HTML.replace("</body>", &format!("{}</body>", inject));
    let _ = wv.load_html(&html);
    refresh_tab_bar(state);
}

fn load_ntp_html(state: &Rc<RefCell<AppState>>) -> String {
    let (bg_js, search_prefix, ntp_json) = {
        let s = state.borrow();
        (
            bg_inject_js(&s.settings),
            search_url_prefix(&s.settings.search_engine).to_string(),
            serde_json::to_string(&s.settings.ntp).unwrap_or_default(),
        )
    };
    let mut inject = format!(
        "<script>window.__SEARCH_URL__='{}';window.__NTP__={};",
        search_prefix, ntp_json
    );
    inject.push_str(&bg_js);
    inject.push_str("</script>\n<script>");
    NTP_HTML.replace("<script>", &inject)
}

fn close_tab_at(state: &Rc<RefCell<AppState>>, idx: usize) {
    {
        let mut s = state.borrow_mut();
        if s.tabs.len() <= 1 {
            s.tabs[0] = Tab {
                id: "start".into(),
                url: String::new(),
                title: "New Tab".into(),
                active: true,
            };
            s.active_idx = 0;
        } else {
            s.tabs.remove(idx);
            if s.active_idx >= s.tabs.len() {
                s.active_idx = s.tabs.len() - 1;
            } else if s.active_idx > idx {
                s.active_idx -= 1;
            }
        }
    }

    let url = state.borrow().tabs[state.borrow().active_idx].url.clone();
    let wv = get_wv(state);
    if url.is_empty() {
        let html = load_ntp_html(state);
        let _ = wv.load_html(&html);
    } else {
        let _ = wv.load_url(&url);
    }
    refresh_tab_bar(state);
}

fn new_tab(state: &Rc<RefCell<AppState>>, url: &str) {
    {
        let mut s = state.borrow_mut();
        for t in s.tabs.iter_mut() {
            t.active = false;
        }
        let id = uuid::Uuid::new_v4().to_string();
        let short_id = id[..8].to_string();
        let title = if url.is_empty() {
            "New Tab".to_string()
        } else {
            url::Url::parse(url)
                .map(|u| u.host_str().unwrap_or("").to_string())
                .unwrap_or_default()
        };
        s.tabs.push(Tab {
            id: short_id,
            url: url.to_string(),
            title,
            active: !url.is_empty(),
        });
        s.active_idx = s.tabs.len() - 1;
    }
    let wv = get_wv(state);
    if url.is_empty() {
        let html = load_ntp_html(state);
        let _ = wv.load_html(&html);
    } else {
        let _ = wv.load_url(url);
    }
    refresh_tab_bar(state);
}

pub fn main() {
    // logging disabled
    gtk::init().unwrap();

    let css = gtk::CssProvider::new();
    css.load_from_data(b"
        window { background: #1a1a1a; }
        .toolbar { background: #2b2b2b; }
        .nav-btn { background: transparent; border-radius: 50%; padding: 4px; min-width: 30px; min-height: 30px; color: #9ca3af; }
        .nav-btn:hover { background: rgba(255,255,255,0.08); color: #d1d5db; }
        .url-entry { background: #3b3b3b; border: 1px solid #444; border-radius: 20px; padding: 0 14px; color: #e5e7eb; font-size: 13px; min-height: 30px; }
        .url-entry:focus { border-color: #666; }
        .url-entry selection { background: rgba(100,100,100,0.4); }
        .menu-btn { background: transparent; border-radius: 50%; min-width: 30px; min-height: 30px; color: #9ca3af; }
        .menu-btn:hover { background: rgba(255,255,255,0.08); color: #d1d5db; }
        .home-btn { background: transparent; border-radius: 50%; min-width: 30px; min-height: 30px; color: #9ca3af; }
        .home-btn:hover { background: rgba(255,255,255,0.08); color: #d1d5db; }
        .new-tab-btn { background: transparent; border-radius: 50%; min-width: 28px; min-height: 28px; color: #9ca3af; font-size: 18px; }
        .new-tab-btn:hover { background: rgba(255,255,255,0.08); color: #d1d5db; }
        .tab-bar { background: #2b2b2b; }
        .tab-btn { background: transparent; border-radius: 8px 8px 0 0; padding: 4px 10px; min-height: 30px; margin: 0 1px; }
        .tab-btn:hover { background: rgba(255,255,255,0.05); }
        .tab-btn label { color: #9ca3af; font-size: 11px; }
        .active-tab { background: #1a1a1a; border-radius: 8px 8px 0 0; }
        .active-tab label { color: #e5e7eb; }
        .tab-close { background: transparent; border-radius: 50%; min-width: 18px; min-height: 18px; padding: 0; color: #6b7280; font-size: 10px; }
        .tab-close:hover { background: rgba(255,255,255,0.12); color: #d1d5db; }
        .menu-popover { background: #2b2b2b; border: 1px solid #3b3b3b; border-radius: 10px; padding: 8px; }
        .menu-popover button { padding: 6px 12px; border-radius: 6px; color: #d1d5db; }
        .menu-popover button:hover { background: rgba(255,255,255,0.08); }
        .dim-label { color: #6b7280; font-size: 11px; padding: 4px 12px; }
        separator { background: #3b3b3b; min-height: 1px; }
    ").unwrap();
    gtk::StyleContext::add_provider_for_screen(
        &gtk::gdk::Screen::default().expect("Could not get default screen"),
        &css,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let settings = Settings::load();

    let event_loop = EventLoopBuilder::new().build();
    let window = WindowBuilder::new()
        .with_title("Ruva Browser")
        .with_inner_size(wry::dpi::LogicalSize::new(1280.0, 800.0))
        .with_min_inner_size(wry::dpi::LogicalSize::new(800.0, 600.0))
        .build(&event_loop)
        .unwrap();

    let gtk_window = window.gtk_window();

    let logo_path = std::env::current_exe().ok()
        .and_then(|p| p.parent().map(|d| d.join("newlogo.png")))
        .filter(|p| p.exists())
        .or_else(|| {
            let p = PathBuf::from("/usr/share/ruva-browser/newlogo.png");
            if p.exists() { Some(p) } else { None }
        })
        .or_else(|| {
            let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("newlogo.png");
            if p.exists() { Some(p) } else { None }
        });
    if let Some(ref logo) = logo_path {
        let _ = gtk_window.set_icon_from_file(logo);
    }
    let container: gtk::Box = gtk_window
        .children()
        .into_iter()
        .find_map(|w| w.downcast::<gtk::Box>().ok())
        .expect("tao GtkBox not found");

    let toolbar = gtk::Box::new(gtk::Orientation::Vertical, 0);
    toolbar.style_context().add_class("toolbar");
    container.pack_start(&toolbar, false, false, 0);

    // Row 1: hamburger + nav + URL + home + new tab
    let row1 = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    row1.set_margin_start(6);
    row1.set_margin_end(6);
    row1.set_margin_top(6);
    row1.set_margin_bottom(4);
    toolbar.pack_start(&row1, false, false, 0);

    // Hamburger
    let btn_menu = gtk::MenuButton::new();
    btn_menu.set_label("\u{2630}");
    btn_menu.set_size_request(32, 30);
    btn_menu.style_context().add_class("menu-btn");
    row1.pack_start(&btn_menu, false, false, 0);

    // Back
    let btn_back = gtk::Button::with_label("\u{2190}");
    btn_back.set_size_request(32, 30);
    btn_back.style_context().add_class("nav-btn");
    row1.pack_start(&btn_back, false, false, 0);

    // Forward
    let btn_fwd = gtk::Button::with_label("\u{2192}");
    btn_fwd.set_size_request(32, 30);
    btn_fwd.style_context().add_class("nav-btn");
    row1.pack_start(&btn_fwd, false, false, 0);

    // Reload
    let btn_reload = gtk::Button::with_label("\u{21bb}");
    btn_reload.set_size_request(32, 30);
    btn_reload.style_context().add_class("nav-btn");
    row1.pack_start(&btn_reload, false, false, 0);

    // Home
    let btn_home = gtk::Button::with_label("\u{2302}");
    btn_home.set_size_request(32, 30);
    btn_home.set_tooltip_text(Some("Домой"));
    btn_home.style_context().add_class("home-btn");
    row1.pack_start(&btn_home, false, false, 0);

    // URL
    let url_entry = gtk::Entry::new();
    url_entry.set_placeholder_text(Some("Поиск или URL..."));
    url_entry.set_hexpand(true);
    url_entry.style_context().add_class("url-entry");
    row1.pack_start(&url_entry, true, true, 0);

    // New tab
    let btn_new_tab = gtk::Button::with_label("+");
    btn_new_tab.set_size_request(30, 30);
    btn_new_tab.style_context().add_class("new-tab-btn");
    row1.pack_start(&btn_new_tab, false, false, 0);

    // Tab bar
    let tab_revealer = gtk::Revealer::new();
    tab_revealer.set_transition_type(gtk::RevealerTransitionType::SlideDown);
    tab_revealer.set_transition_duration(200);
    toolbar.pack_start(&tab_revealer, false, false, 0);

    let tab_scroll = gtk::ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
    tab_scroll.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Never);
    tab_scroll.set_min_content_height(36);
    tab_scroll.set_max_content_height(36);
    tab_scroll.style_context().add_class("tab-bar");
    tab_revealer.add(&tab_scroll);

    let tab_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    tab_box.set_margin_start(60);
    tab_scroll.add(&tab_box);

    let sep = gtk::Separator::new(gtk::Orientation::Horizontal);
    toolbar.pack_start(&sep, false, false, 0);

    tab_revealer.set_reveal_child(settings.show_tab_bar);

    // State
    let state = Rc::new(RefCell::new(AppState {
        tabs: vec![Tab {
            id: "start".into(),
            url: String::new(),
            title: "New Tab".into(),
            active: true,
        }],
        active_idx: 0,
        webview: None,
        tab_revealer: Some(tab_revealer.clone()),
        url_entry: Some(url_entry.clone()),
        settings: settings.clone(),
        on_settings_page: false,
    }));

    // WebContext with cookies
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let data_dir = PathBuf::from(&home).join(DATA_DIR);
    let _ = std::fs::create_dir_all(&data_dir);
    let mut web_context = WebContext::new(Some(data_dir));

    let (ipc_tx, ipc_rx) = mpsc::channel::<String>();
    let (ai_tx, ai_rx) = mpsc::channel::<String>();
    let (settings_tx, settings_rx) = mpsc::channel::<String>();

    let fullscreen_js = if settings.block_fullscreen {
        FULLSCREEN_BLOCK_JS
    } else {
        ""
    };

    let webview = WebViewBuilder::with_web_context(&mut web_context)
        .with_html(NTP_HTML)
        .with_initialization_script(fullscreen_js)
        .with_user_agent(USER_AGENT)
        .with_ipc_handler(move |req: Request<String>| {
            let _ = ipc_tx.send(req.body().to_string());
        })
        .build_gtk(&container)
        .unwrap();

    let webview = Rc::new(webview);
    state.borrow_mut().webview = Some(webview.clone());

    // ====== HAMBURGER MENU ======
    {
        let popover = gtk::Popover::new(Some(&btn_menu));
        popover.set_position(gtk::PositionType::Bottom);
        popover.style_context().add_class("menu-popover");

        let menu_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
        popover.add(&menu_box);

        // Tabs section
        let tabs_header = gtk::Label::new(Some("Вкладки"));
        tabs_header.set_xalign(0.0);
        tabs_header.style_context().add_class("dim-label");
        menu_box.pack_start(&tabs_header, false, false, 0);

        let tabs_list_box = gtk::Box::new(gtk::Orientation::Vertical, 2);
        menu_box.pack_start(&tabs_list_box, false, false, 0);

        let state_clone = state.clone();
        let popover_rc = Rc::new(popover.clone());
        let popover_rc_for_menu = popover_rc.clone();
        let tabs_list_clone = tabs_list_box.clone();
        btn_menu.connect_clicked(move |_btn| {
            for child in tabs_list_clone.children() {
                tabs_list_clone.remove(&child);
            }
            {
                let s = state_clone.borrow();
                for (i, tab) in s.tabs.iter().enumerate() {
                    let title = if tab.title.len() > 30 {
                        format!("{}...", &tab.title[..27])
                    } else {
                        tab.title.clone()
                    };
                    let prefix = if i == s.active_idx { "\u{25b6} " } else { "  " };
                    let btn = gtk::Button::new();
                    let lbl = gtk::Label::new(Some(&format!("{}{}", prefix, title)));
                    lbl.set_xalign(0.0);
                    lbl.set_ellipsize(gtk::pango::EllipsizeMode::End);
                    lbl.set_max_width_chars(35);
                    btn.add(&lbl);
                    btn.set_relief(gtk::ReliefStyle::None);

                    let s2 = state_clone.clone();
                    let p2 = popover_rc_for_menu.clone();
                    let idx = i;
                    btn.connect_clicked(move |_| {
                        let url = {
                            let mut st = s2.borrow_mut();
                            if idx < st.tabs.len() {
                                st.active_idx = idx;
                                st.tabs[idx].url.clone()
                            } else {
                                return;
                            }
                        };
                        let wv = get_wv(&s2);
                        if url.is_empty() {
                            let _ = wv.load_html(NTP_HTML);
                        } else {
                            let _ = wv.load_url(&url);
                        }
                        refresh_tab_bar(&s2);
                        p2.popdown();
                    });
                    tabs_list_clone.pack_start(&btn, false, false, 0);
                }
            }
            tabs_list_clone.show_all();
        });

        let sep2 = gtk::Separator::new(gtk::Orientation::Horizontal);
        menu_box.pack_start(&sep2, false, false, 4);

        // Settings
        let settings_header = gtk::Label::new(Some("Настройки"));
        settings_header.set_xalign(0.0);
        settings_header.style_context().add_class("dim-label");
        menu_box.pack_start(&settings_header, false, false, 0);

        let open_settings_btn = gtk::Button::with_label("\u{2699}  Открыть настройки");
        open_settings_btn.set_relief(gtk::ReliefStyle::None);

        menu_box.pack_start(&open_settings_btn, false, false, 0);
        {
            let s = state.clone();
            let p = popover_rc.clone();
            open_settings_btn.connect_clicked(move |_| {
                load_settings_page(&s);
                p.popdown();
            });
        }

        let auto_show_cb = gtk::CheckButton::with_label("Автопоказ панели при наведении");
        auto_show_cb.set_active(state.borrow().settings.auto_show_tab_bar);
        menu_box.pack_start(&auto_show_cb, false, false, 0);
        {
            let s = state.clone();
            let ts = tab_revealer.clone();
            auto_show_cb.connect_toggled(move |cb| {
                let mut st = s.borrow_mut();
                st.settings.auto_show_tab_bar = cb.is_active();
                st.settings.save();
                drop(st);
                let st2 = s.borrow();
                ts.set_reveal_child(st2.settings.auto_show_tab_bar);
            });
        }

        let show_tabs_cb = gtk::CheckButton::with_label("Показывать панель вкладок");
        show_tabs_cb.set_active(state.borrow().settings.show_tab_bar);
        menu_box.pack_start(&show_tabs_cb, false, false, 0);
        {
            let s = state.clone();
            let ts = tab_revealer.clone();
            show_tabs_cb.connect_toggled(move |cb| {
                let mut st = s.borrow_mut();
                st.settings.show_tab_bar = cb.is_active();
                st.settings.save();
                drop(st);
                let st2 = s.borrow();
                ts.set_reveal_child(st2.settings.show_tab_bar);
            });
        }

        let sep3 = gtk::Separator::new(gtk::Orientation::Horizontal);
        menu_box.pack_start(&sep3, false, false, 4);

        let clear_btn = gtk::Button::with_label("\u{1f5d1}  Очистить куки и данные");
        clear_btn.set_relief(gtk::ReliefStyle::None);

        menu_box.pack_start(&clear_btn, false, false, 0);
        {
            let popover4 = popover.clone();
            clear_btn.connect_clicked(move |_| {
                let data_dir = PathBuf::from(
                    std::env::var("HOME").unwrap_or_else(|_| ".".into())
                ).join(DATA_DIR);
                let _ = std::fs::remove_dir_all(&data_dir);
                let _ = std::fs::create_dir_all(&data_dir);
                popover4.popdown();
            });
        }

        menu_box.show_all();
        btn_menu.set_popover(Some(&popover));
    }

    // Show everything
    toolbar.show_all();
    gtk_window.show_all();

    // Home button
    {
        let s = state.clone();
        btn_home.connect_clicked(move |_| {
            let url = {
                let st = s.borrow();
                st.settings.homepage.clone()
            };
            if url.is_empty() || url == "ruva://newtab" {
                load_ntp(&s);
            } else {
                navigate_to(&s, &url);
            }
        });
    }

    // URL entry: Enter to navigate
    {
        let s = state.clone();
        url_entry.connect_activate(move |e| {
            let val = e.text().to_string();
            if val.is_empty() { return; }
            let url = {
                let st = s.borrow();
                normalize_url(&st, &val)
            };
            navigate_to(&s, &url);
            e.grab_focus();
        });
    }

    // Back / Forward / Reload
    {
        let wv = webview.clone();
        btn_back.connect_clicked(move |_| { let _ = wv.evaluate_script("history.back()"); });
    }
    {
        let wv = webview.clone();
        btn_fwd.connect_clicked(move |_| { let _ = wv.evaluate_script("history.forward()"); });
    }
    {
        let wv = webview.clone();
        btn_reload.connect_clicked(move |_| { let _ = wv.evaluate_script("location.reload()"); });
    }

    // New tab
    {
        let s = state.clone();
        let entry = url_entry.clone();
        btn_new_tab.connect_clicked(move |_| {
            new_tab(&s, "");
            entry.set_text("");
            entry.grab_focus();
        });
    }

    // Keyboard shortcuts
    let kb_state = state.clone();
    let kb_entry = url_entry.clone();
    let kb_webview = webview.clone();
    let mut ctrl_held = false;

    refresh_tab_bar(&state);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        while let Ok(json_str) = settings_rx.try_recv() {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json_str) {
                let mut st = kb_state.borrow_mut();
                if let Some(v) = val.get("search_engine").and_then(|x| x.as_str()) {
                    if !v.is_empty() { st.settings.search_engine = v.to_string(); }
                }
                if let Some(v) = val.get("homepage").and_then(|x| x.as_str()) {
                    st.settings.homepage = v.to_string();
                }
                if let Some(v) = val.get("bg_color").and_then(|x| x.as_str()) {
                    st.settings.bg_color = v.to_string();
                }
                if let Some(v) = val.get("bg_image").and_then(|x| x.as_str()) {
                    st.settings.bg_image = v.to_string();
                }
                if let Some(v) = val.get("bg_video").and_then(|x| x.as_str()) {
                    st.settings.bg_video = v.to_string();
                }
                if let Some(v) = val.get("new_tab_show_ntp").and_then(|x| x.as_bool()) {
                    st.settings.new_tab_show_ntp = v;
                }
                if let Some(v) = val.get("show_tab_bar").and_then(|x| x.as_bool()) {
                    st.settings.show_tab_bar = v;
                }
                if let Some(v) = val.get("auto_show_tab_bar").and_then(|x| x.as_bool()) {
                    st.settings.auto_show_tab_bar = v;
                }
                if let Some(v) = val.get("block_fullscreen").and_then(|x| x.as_bool()) {
                    st.settings.block_fullscreen = v;
                }
                if let Some(v) = val.get("load_images").and_then(|x| x.as_bool()) {
                    st.settings.load_images = v;
                }
                if let Some(v) = val.get("ai_api_key").and_then(|x| x.as_str()) {
                    if !v.is_empty() { st.settings.ai_api_key = v.to_string(); }
                }
                if let Some(v) = val.get("ai_model").and_then(|x| x.as_str()) {
                    st.settings.ai_model = v.to_string();
                }
                if let Some(ntp) = val.get("ntp") {
                    if let Ok(ntp_settings) = serde_json::from_value::<NtpSettings>(ntp.clone()) {
                        st.settings.ntp = ntp_settings;
                    }
                }
                st.settings.save();
            }
        }

        while let Ok(text) = ai_rx.try_recv() {
            let safe = text.replace('\\', "\\\\")
                .replace('`', "\\`")
                .replace('\n', "<br>")
                .replace('\r', "")
                .replace('\'', "\\'")
                .replace('"', "&quot;")
                .replace('<', "&lt;")
                .replace('>', "&gt;");
            let js = format!(
                "if(document.getElementById('aiResponse')){{document.getElementById('aiResponse').innerHTML='{}';document.getElementById('aiLoading').style.display='none';document.getElementById('aiResponse').style.display='block';}}",
                safe
            );
            let _ = kb_webview.evaluate_script(&js);
        }

        while let Ok(msg_str) = ipc_rx.try_recv() {
            if let Ok(msg) = serde_json::from_str::<IpcMsg>(&msg_str) {
                match msg.cmd.as_str() {
                    "navigate" => {
                        if !msg.url.is_empty() {
                            navigate_to(&kb_state, &msg.url);
                        } else if kb_state.borrow().on_settings_page {
                            let wv = get_wv(&kb_state);
                            let tx = settings_tx.clone();
                            let _ = wv.evaluate_script_with_callback(
                                "JSON.stringify(collectSettings())",
                                move |result| { let _ = tx.send(result); },
                            );
                            load_ntp(&kb_state);
                        } else {
                            load_ntp(&kb_state);
                        }
                    }
                    "set_title" => {
                        let mut st = kb_state.borrow_mut();
                        let idx = st.active_idx;
                        if let Some(tab) = st.tabs.get_mut(idx) {
                            tab.title = msg.title.clone();
                        }
                        drop(st);
                        refresh_tab_bar(&kb_state);
                    }
                    "save_settings" => {
                        let mut st = kb_state.borrow_mut();
                        if !msg.search_engine.is_empty() {
                            st.settings.search_engine = msg.search_engine.clone();
                        }
                        if let Some(ref hp) = msg.homepage {
                            st.settings.homepage = hp.clone();
                        }
                        if let Some(ref c) = msg.bg_color {
                            st.settings.bg_color = c.clone();
                        }
                        if let Some(ref i) = msg.bg_image {
                            st.settings.bg_image = i.clone();
                        }
                        if let Some(ref v) = msg.bg_video {
                            st.settings.bg_video = v.clone();
                        }
                        if let Some(v) = msg.new_tab_show_ntp {
                            st.settings.new_tab_show_ntp = v;
                        }
                        if let Some(v) = msg.show_tab_bar {
                            st.settings.show_tab_bar = v;
                        }
                        if let Some(v) = msg.auto_show_tab_bar {
                            st.settings.auto_show_tab_bar = v;
                        }
                        if let Some(v) = msg.block_fullscreen {
                            st.settings.block_fullscreen = v;
                        }
                        if let Some(v) = msg.load_images {
                            st.settings.load_images = v;
                        }
                        if let Some(ref ntp) = msg.ntp {
                            st.settings.ntp = ntp.clone();
                        }
                        if let Some(ref k) = msg.ai_api_key {
                            if !k.is_empty() {
                                st.settings.ai_api_key = k.clone();
                            }
                        }
                        if let Some(ref m) = msg.ai_model {
                            st.settings.ai_model = m.clone();
                        }
                        st.settings.save();
                    }
                    "ai_chat" => {
                        let prompt = msg.ai_prompt.clone();
                        let api_key = {
                            let s = state.borrow();
                            let k = s.settings.ai_api_key.clone();
                            if k.is_empty() { HARDCODED_API_KEY.to_string() } else { k }
                        };
                        let model = state.borrow().settings.ai_model.clone();
                        let ai_tx_clone = ai_tx.clone();
                        std::thread::spawn(move || {
                            if api_key.is_empty() {
                                let _ = ai_tx_clone.send("\u{26a0}\u{fe0f} Введите API ключ OpenRouter в настройках".to_string());
                                return;
                            }
                            let body = serde_json::json!({
                                "model": model,
                                "messages": [{"role": "user", "content": prompt}],
                                "max_tokens": 256,
                            });
                            let body_str = serde_json::to_string(&body).unwrap_or_default();
                            let tmp = std::env::temp_dir().join("ruva_ai_body.json");
                            let _ = std::fs::write(&tmp, &body_str);
                            let output = std::process::Command::new("curl")
                                .args([
                                    "-s", "--max-time", "30",
                                    "-X", "POST",
                                    "https://openrouter.ai/api/v1/chat/completions",
                                    "-H", &format!("Authorization: Bearer {}", api_key),
                                    "-H", "Content-Type: application/json",
                                    "-d", &body_str,
                                ])
                                .output();
                            let _ = std::fs::remove_file(&tmp);
                            let text = match output {
                                Ok(out) if out.status.success() => {
                                    let raw = String::from_utf8_lossy(&out.stdout).to_string();
                                    match serde_json::from_str::<serde_json::Value>(&raw) {
                                        Ok(v) => {
                                            let msg = &v["choices"][0]["message"];
                                            let content = msg["content"].as_str();
                                            let reasoning = msg["reasoning"].as_str();
                                            if let Some(c) = content {
                                                c.to_string()
                                            } else if let Some(r) = reasoning {
                                                r.to_string()
                                            } else if let Some(e) = v["error"]["message"].as_str() {
                                                format!("\u{26a0}\u{fe0f} {}", e)
                                            } else {
                                                raw
                                            }
                                        }
                                        Err(_) => raw,
                                    }
                                }
                                Ok(out) => {
                                    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                                    format!("\u{26a0}\u{fe0f} Ошибка: {}", stderr)
                                }
                                Err(_) => "\u{26a0}\u{fe0f} Ошибка соединения".to_string(),
                            };
                            let _ = ai_tx_clone.send(text);
                        });
                    }
                    "clear_data" => {
                        let data_dir = PathBuf::from(
                            std::env::var("HOME").unwrap_or_else(|_| ".".into())
                        ).join(DATA_DIR);
                        let _ = std::fs::remove_dir_all(&data_dir);
                        let _ = std::fs::create_dir_all(&data_dir);
                    }
                    _ => {}
                }
            }
        }

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                WindowEvent::ModifiersChanged(state) => {
                    ctrl_held = state.control_key();
                }
                WindowEvent::KeyboardInput { event: key_event, .. } => {
                    if key_event.state == tao::event::ElementState::Pressed && ctrl_held {
                        match key_event.logical_key {
                            tao::keyboard::Key::Character("t") | tao::keyboard::Key::Character("T") => {
                                new_tab(&kb_state, "");
                                kb_entry.set_text("");
                                kb_entry.grab_focus();
                            }
                            tao::keyboard::Key::Character("w") | tao::keyboard::Key::Character("W") => {
                                let idx = kb_state.borrow().active_idx;
                                close_tab_at(&kb_state, idx);
                            }
                            tao::keyboard::Key::Character("l") | tao::keyboard::Key::Character("L") => {
                                kb_entry.grab_focus();
                                kb_entry.select_region(0, -1);
                            }
                            tao::keyboard::Key::Character("r") | tao::keyboard::Key::Character("R") => {
                                let wv = get_wv(&kb_state);
                                let _ = wv.evaluate_script("location.reload()");
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        }
    });
}
