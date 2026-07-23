use std::cell::RefCell;
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::WindowBuilder;

const TOOLBAR_JS: &str = include_str!("../ui/toolbar_inject.js");
const NTP_HTML: &str = include_str!("../ui/ntp.html");
const SETTINGS_HTML: &str = include_str!("../ui/settings.html");
const DATA_DIR: &str = ".ruva";
const HARDCODED_API_KEY: &str = "";

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
    #[serde(default)] url: String,
    #[serde(default)] title: String,
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
            ],
        }
    }
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
    ai_api_key: String,
    ai_model: String,
    ntp: NtpSettings,
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
            ai_model: "openrouter/free".into(),
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

struct AppState {
    tabs: Vec<Tab>,
    active_idx: usize,
    settings: Settings,
    is_ntp: bool,
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
    format!("{}{}", search_url_prefix(&settings.search_engine), trimmed)
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
        js.push_str(&format!(
            "document.body.style.background=\"url('{}') no-repeat center center fixed\";",
            escaped
        ));
        js.push_str("document.body.style.backgroundSize='cover';");
    }
    if !settings.bg_video.is_empty() {
        let escaped = settings.bg_video.replace('\\', "\\\\").replace('\'', "\\'");
        js.push_str(&format!(
            "(function(){{var v=document.createElement('video');v.src='{}';v.autoplay=true;v.loop=true;v.muted=true;v.style.cssText='position:fixed;top:0;left:0;width:100%;height:100%;object-fit:cover;z-index:-1';document.body.prepend(v)}})();",
            escaped
        ));
    }
    js
}

fn build_ntp_html(settings: &Settings) -> String {
    let bg_js = bg_inject_js(settings);
    let search_prefix = search_url_prefix(&settings.search_engine);
    let ntp_json = serde_json::to_string(&settings.ntp).unwrap_or_default();
    let mut inject = format!(
        "<script>window.__SEARCH_URL__='{}';window.__NTP__={};",
        search_prefix, ntp_json
    );
    inject.push_str(&bg_js);
    inject.push_str("</script>\n<script>");
    NTP_HTML.replace("<script>", &inject)
}

fn build_settings_html(settings: &Settings) -> String {
    let settings_json = serde_json::to_string(settings).unwrap_or_default();
    let inject = format!("<script>window.__SETTINGS__={};</script>", settings_json);
    SETTINGS_HTML.replace("</body>", &format!("{}</body>", inject))
}

// --- IPC via local HTTP server ---

struct IpcQueue {
    pending: Mutex<VecDeque<String>>,
}

impl IpcQueue {
    fn new() -> Self {
        IpcQueue {
            pending: Mutex::new(VecDeque::new()),
        }
    }
    fn push(&self, msg: String) {
        self.pending.lock().unwrap().push_back(msg);
    }
    fn drain(&self) -> Vec<String> {
        self.pending.lock().unwrap().drain(..).collect()
    }
}

fn start_ipc_server(queue: Arc<IpcQueue>) -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            if let Ok(mut stream) = stream {
                let mut buf = [0u8; 65536];
                let n = stream.read(&mut buf).unwrap_or(0);
                let request = String::from_utf8_lossy(&buf[..n]);
                if request.starts_with("POST /ipc") {
                    if let Some(body_start) = request.find("\r\n\r\n") {
                        let body = &request[body_start + 4..];
                        queue.push(body.to_string());
                        let response = "HTTP/1.1 200 OK\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: 2\r\n\r\n{}";
                        stream.write_all(response.as_bytes()).ok();
                    }
                } else if request.starts_with("OPTIONS") {
                    let response = "HTTP/1.1 204 OK\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nContent-Length: 0\r\n\r\n";
                    stream.write_all(response.as_bytes()).ok();
                } else {
                    let response = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
                    stream.write_all(response.as_bytes()).ok();
                }
            }
        }
    });
    port
}

// --- IPC bridge JS: overrides window.ipc.postMessage to POST to local server ---

fn ipc_bridge_js(port: u16) -> String {
    format!(
        r#"
window.__ruvaPort={};
window.ipc={{postMessage:function(data){{fetch('http://127.0.0.1:'+window.__ruvaPort+'/ipc',{{method:'POST',body:typeof data==='string'?data:JSON.stringify(data),headers:{{'Content-Type':'application/json'}}}}).catch(function(){{}});}}}};
"#,
        port
    )
}

pub fn main() {
    let settings = Settings::load();
    let data_dir = PathBuf::from(
        std::env::var("LOCALAPPDATA")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".into()),
    )
    .join(DATA_DIR);
    let _ = std::fs::create_dir_all(&data_dir);

    let ipc_queue = Arc::new(IpcQueue::new());
    let ipc_port = start_ipc_server(ipc_queue.clone());

    let event_loop = EventLoopBuilder::new().build();

    let window = WindowBuilder::new()
        .with_title("Ruva Brower Windows edition")
        .with_inner_size(tao::dpi::LogicalSize::new(1280.0, 800.0))
        .with_min_inner_size(tao::dpi::LogicalSize::new(800.0, 600.0))
        .build(&event_loop)
        .unwrap();

    // Channel: main thread -> chromium backend
    let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<String>();
    // Channel: AI response -> main thread
    let (ai_tx, ai_rx) = std::sync::mpsc::channel::<String>();

    let state = Rc::new(RefCell::new(AppState {
        tabs: vec![Tab {
            id: "start".into(),
            url: String::new(),
            title: "New Tab".into(),
            active: true,
        }],
        active_idx: 0,
        settings,
        is_ntp: true,
    }));

    let initial_html = build_ntp_html(&state.borrow().settings);
    let bridge_js = ipc_bridge_js(ipc_port);

    // Spawn chromium backend
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(chromium_backend(initial_html, bridge_js, cmd_rx, data_dir.clone()));
    });

    let kb_state = state.clone();
    let kb_cmd_tx = cmd_tx.clone();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        // Process IPC messages from JS via HTTP server queue
        for msg_str in ipc_queue.drain() {
            if let Ok(msg) = serde_json::from_str::<IpcMsg>(&msg_str) {
                match msg.cmd.as_str() {
                    "navigate" => {
                        let mut st = kb_state.borrow_mut();
                        if !msg.url.is_empty() {
                            let url = normalize_url(&msg.url, &st.settings);
                            st.is_ntp = false;
                            let idx = st.active_idx;
                            if let Some(tab) = st.tabs.get_mut(idx) {
                                tab.url = url.clone();
                                tab.title = url::Url::parse(&url)
                                    .map(|u| u.host_str().unwrap_or("").to_string())
                                    .unwrap_or_default();
                            }
                            drop(st);
                            let _ = kb_cmd_tx.send(
                                serde_json::json!({"cmd":"navigate","url":url}).to_string(),
                            );
                        } else {
                            st.is_ntp = true;
                            let idx = st.active_idx;
                            if let Some(tab) = st.tabs.get_mut(idx) {
                                tab.url.clear();
                                tab.title = "New Tab".into();
                            }
                            let html = build_ntp_html(&st.settings);
                            drop(st);
                            let _ = kb_cmd_tx.send(
                                serde_json::json!({"cmd":"load_html","html":html}).to_string(),
                            );
                        }
                    }
                    "set_title" => {
                        let mut st = kb_state.borrow_mut();
                        let idx = st.active_idx;
                        if let Some(tab) = st.tabs.get_mut(idx) {
                            tab.title = msg.title.clone();
                        }
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
                    "open_settings" => {
                        let mut st = kb_state.borrow_mut();
                        st.is_ntp = false;
                        let html = build_settings_html(&st.settings);
                        drop(st);
                        let _ = kb_cmd_tx.send(
                            serde_json::json!({"cmd":"load_html","html":html}).to_string(),
                        );
                    }
                    "ai_chat" => {
                        let prompt = msg.ai_prompt.clone();
                        let api_key = {
                            let s = kb_state.borrow();
                            let k = s.settings.ai_api_key.clone();
                            if k.is_empty() {
                                HARDCODED_API_KEY.to_string()
                            } else {
                                k
                            }
                        };
                        let model = kb_state.borrow().settings.ai_model.clone();
                        let ai_tx_clone = ai_tx.clone();
                        std::thread::spawn(move || {
                            if api_key.is_empty() {
                                let _ = ai_tx_clone
                                    .send("\u{26a0}\u{fe0f} API key not set".to_string());
                                return;
                            }
                            let body = serde_json::json!({
                                "model": model,
                                "messages": [{"role": "user", "content": prompt}],
                                "max_tokens": 256
                            });
                            let body_str = serde_json::to_string(&body).unwrap_or_default();
                            let output = std::process::Command::new("curl")
                                .args([
                                    "-s", "--max-time", "30", "-X", "POST",
                                    "https://openrouter.ai/api/v1/chat/completions",
                                    "-H", &format!("Authorization: Bearer {}", api_key),
                                    "-H", "Content-Type: application/json",
                                    "-d", &body_str,
                                ])
                                .output();
                            let text = match output {
                                Ok(out) if out.status.success() => {
                                    let raw = String::from_utf8_lossy(&out.stdout).to_string();
                                    match serde_json::from_str::<serde_json::Value>(&raw) {
                                        Ok(v) => {
                                            let msg2 = &v["choices"][0]["message"];
                                            if let Some(c) = msg2["content"].as_str() {
                                                c.to_string()
                                            } else if let Some(r) =
                                                msg2["reasoning"].as_str()
                                            {
                                                r.to_string()
                                            } else if let Some(e) =
                                                v["error"]["message"].as_str()
                                            {
                                                format!("\u{26a0}\u{fe0f} {}", e)
                                            } else {
                                                raw
                                            }
                                        }
                                        Err(_) => raw,
                                    }
                                }
                                Ok(out) => format!(
                                    "\u{26a0}\u{fe0f} {}",
                                    String::from_utf8_lossy(&out.stderr)
                                ),
                                Err(_) => "\u{26a0}\u{fe0f} Connection error".to_string(),
                            };
                            let _ = ai_tx_clone.send(text);
                        });
                    }
                    "clear_data" => {
                        let _ = std::fs::remove_dir_all(&data_dir);
                        let _ = std::fs::create_dir_all(&data_dir);
                    }
                    _ => {}
                }
            }
        }

        // Process AI responses
        while let Ok(text) = ai_rx.try_recv() {
            let safe = text
                .replace('\\', "\\\\")
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
            let _ = kb_cmd_tx.send(
                serde_json::json!({"cmd":"execute_js","js":js}).to_string(),
            );
        }

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    let _ = kb_cmd_tx.send(
                        serde_json::json!({"cmd":"shutdown"}).to_string(),
                    );
                    *control_flow = ControlFlow::Exit;
                }
                _ => {}
            },
            _ => {}
        }
    });
}

// --- Chromium backend (runs on dedicated tokio thread) ---

use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::fetcher::{BrowserFetcher, BrowserFetcherOptions};
use chromiumoxide::page::Page;
use futures::StreamExt;

async fn chromium_backend(
    initial_html: String,
    bridge_js: String,
    cmd_rx: std::sync::mpsc::Receiver<String>,
    data_dir: PathBuf,
) {
    let fetcher_path = data_dir.join("chromium");
    let _ = std::fs::create_dir_all(&fetcher_path);

    let fetcher = BrowserFetcher::new(
        BrowserFetcherOptions::builder()
            .with_path(&fetcher_path)
            .build()
            .unwrap(),
    );

    let info = match fetcher.fetch().await {
        Ok(i) => i,
        Err(e) => {
            eprintln!("Download error: {}", e);
            return;
        }
    };

    let config = match BrowserConfig::builder()
        .chrome_executable(info.executable_path)
        .with_head()
        .args([
            "--disable-background-networking",
            "--disable-default-apps",
            "--disable-extensions",
            "--disable-sync",
            "--no-first-run",
        ])
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Config error: {}", e);
            return;
        }
    };

    let (mut browser, mut handler) = match Browser::launch(config).await {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Launch error: {}", e);
            return;
        }
    };

    tokio::spawn(async move {
        while handler.next().await.is_some() {}
    });

    let page = match browser.new_page("about:blank").await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Page error: {}", e);
            return;
        }
    };

    // Inject IPC bridge (toolbar_inject.js calls window.ipc.postMessage)
    let _ = page.evaluate_on_new_document(bridge_js.as_str()).await;

    // Inject toolbar on every new page
    let toolbar_inject = TOOLBAR_JS.replace('\n', " ");
    let _ = page.evaluate_on_new_document(toolbar_inject.as_str()).await;

    // Load initial NTP
    let _ = page.set_content(&initial_html).await;
    let _ = page.evaluate(toolbar_inject.as_str()).await;

    // Poll for commands from main thread
    loop {
        match cmd_rx.try_recv() {
            Ok(cmd_str) => {
                if let Ok(cmd) = serde_json::from_str::<serde_json::Value>(&cmd_str) {
                    match cmd.get("cmd").and_then(|v| v.as_str()) {
                        Some("navigate") => {
                            if let Some(url) = cmd.get("url").and_then(|v| v.as_str()) {
                                let _ = page.goto(url).await;
                                let _ = page.evaluate(toolbar_inject.as_str()).await;
                            }
                        }
                        Some("load_html") => {
                            if let Some(html) = cmd.get("html").and_then(|v| v.as_str()) {
                                let _ = page.goto("about:blank").await;
                                let _ = page.set_content(html).await;
                                let _ = page.evaluate(toolbar_inject.as_str()).await;
                            }
                        }
                        Some("execute_js") => {
                            if let Some(js) = cmd.get("js").and_then(|v| v.as_str()) {
                                let _ = page.evaluate(js).await;
                            }
                        }
                        Some("shutdown") => {
                            browser.close().await.ok();
                            std::process::exit(0);
                        }
                        _ => {}
                    }
                }
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                std::process::exit(0);
            }
            Err(_) => {}
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
}
