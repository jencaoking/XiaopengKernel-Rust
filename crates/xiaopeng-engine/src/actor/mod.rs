use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tracing::info;
use std::sync::Arc;
use futures::future::BoxFuture;

pub mod app;
pub use app::ConstellationApp;

use xiaopeng_dom::NodePtr;
use xiaopeng_renderer::DisplayList;
use xiaopeng_style::parser::StyleSheet;

/// Messages sent to the Script Thread (DOM + JS)
#[derive(Debug)]
pub enum ScriptMsg {
    LoadHtml(String),
    LoadUrl(String),
    Resize(u32, u32),
    DispatchEvent(NodePtr, String),
}

/// Messages sent to the Layout Thread
#[derive(Debug)]
pub enum LayoutMsg {
    Compute {
        root: NodePtr,
        width: f32,
        height: f32,
        stylesheet: Arc<StyleSheet>,
    },
    HitTest {
        x: f32,
        y: f32,
        event_type: String,
        script_tx: UnboundedSender<ScriptMsg>,
    },
}

/// Messages sent to the Render Thread
#[derive(Debug)]
pub enum RenderMsg {
    Render(DisplayList),
}

pub struct EngineActors {
    pub script_tx: UnboundedSender<ScriptMsg>,
    pub layout_tx: UnboundedSender<LayoutMsg>,
    pub render_tx: UnboundedSender<RenderMsg>,
}

impl EngineActors {
    pub fn spawn(config: crate::EngineConfig, initial_doc: Option<xiaopeng_dom::Document>) -> (Self, Option<UnboundedReceiver<RenderMsg>>) {
        let (script_tx, script_rx) = unbounded_channel();
        let (layout_tx, layout_rx) = unbounded_channel();
        let (render_tx, render_rx) = unbounded_channel();

        // If we already have a parsed document from the single-threaded pre-load, kickstart the pipeline
        if let Some(doc) = initial_doc {
            info!("Constellation: Kickstarting actor pipeline with pre-loaded Document");
            if let Err(e) = layout_tx.send(LayoutMsg::Compute {
                root: doc.root,
                width: config.width as f32,
                height: config.height as f32,
                stylesheet: Arc::new(StyleSheet::default()), // Default for now
            }) {
                tracing::error!("Failed to send initial LayoutMsg::Compute: {}", e);
            }
        }

        // Spawn Layout Thread
        let render_tx_clone = render_tx.clone();
        std::thread::spawn(move || {
            Self::layout_loop(layout_rx, render_tx_clone);
        });

        // Spawn Render Thread only if headless
        let mut ui_render_rx = None;
        if config.headless {
            let config_clone = config.clone();
            std::thread::spawn(move || {
                Self::render_loop(render_rx, config_clone);
            });
        } else {
            ui_render_rx = Some(render_rx);
        }

        // Spawn Script Thread (Runs within a Tokio local set or block_on)
        let layout_tx_clone = layout_tx.clone();
        let config_clone2 = config.clone();
        std::thread::spawn(move || {
            // Script thread typically requires a Tokio runtime for network fetch
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Unwrap failed");
            rt.block_on(async {
                Self::script_loop(script_rx, layout_tx_clone, config_clone2).await;
            });
        });

        (Self {
            script_tx,
            layout_tx,
            render_tx,
        }, ui_render_rx)
    }

    async fn script_loop(
        mut rx: UnboundedReceiver<ScriptMsg>,
        layout_tx: UnboundedSender<LayoutMsg>,
        mut config: crate::EngineConfig,
    ) {
        info!("Script Thread started");
        let mut js_runtime = xiaopeng_script::JsRuntime::new().unwrap_or_default();
        let mut current_root: Option<NodePtr> = None;
        let mut ticker = tokio::time::interval(tokio::time::Duration::from_millis(16));
        let net_client = xiaopeng_net::NetClient::new();
        
        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    if xiaopeng_dom::node::take_dom_dirty() {
                        if let Some(ref root) = current_root {
                            info!("Script Thread: DOM mutation detected, triggering incremental layout");
                            if let Err(e) = layout_tx.send(LayoutMsg::Compute {
                                root: NodePtr::clone_ptr(root),
                                width: config.width as f32,
                                height: config.height as f32,
                            }) {
                                tracing::error!("Failed to send LayoutMsg::Compute on mutation: {}", e);
                            }
                        }
                    }
                }
                msg_opt = rx.recv() => {
                    let Some(msg) = msg_opt else { break; };
                    match msg {
                        ScriptMsg::LoadHtml(html) => {
                            info!("Script Thread: Parsing HTML");
                            if let Ok(doc) = xiaopeng_parser::parse_html(&html) {
                                let root_id = xiaopeng_script::bindings::dom::expose_node(NodePtr::clone_ptr(&doc.root));
                                if let Err(e) = js_runtime.eval(&format!("____init_document({});", root_id)) {
                                    tracing::error!("Failed to eval ____init_document: {:?}", e);
                                }
                                current_root = Some(NodePtr::clone_ptr(&doc.root));
                                
                                let sheet = collect_stylesheets(&doc.root, "http://localhost", &net_client).await;
                                
                                if let Err(e) = layout_tx.send(LayoutMsg::Compute {
                                    root: doc.root,
                                    width: config.width as f32,
                                    height: config.height as f32,
                                    stylesheet: Arc::new(sheet),
                                }) {
                                    tracing::error!("Failed to send LayoutMsg::Compute for LoadHtml: {}", e);
                                }
                            }
                        }
                        ScriptMsg::LoadUrl(url) => {
                            info!("Script Thread: Fetching URL {}", url);
                            let req = xiaopeng_net::Request::new("GET", &url);
                            match net_client.fetch(req).await {
                                Ok(res) => {
                                    let html = String::from_utf8_lossy(&res.body).to_string();
                                    info!("Script Thread: URL Fetched ({} bytes). Parsing HTML...", html.len());
                                    if let Ok(doc) = xiaopeng_parser::parse_html(&html) {
                                        let root_id = xiaopeng_script::bindings::dom::expose_node(NodePtr::clone_ptr(&doc.root));
                                        if let Err(e) = js_runtime.eval(&format!("____init_document({});", root_id)) {
                                            tracing::error!("Failed to eval ____init_document: {:?}", e);
                                        }
                                        current_root = Some(NodePtr::clone_ptr(&doc.root));
                                        
                                        let sheet = collect_stylesheets(&doc.root, &url, &net_client).await;
                                        
                                        if let Err(e) = layout_tx.send(LayoutMsg::Compute {
                                            root: doc.root,
                                            width: config.width as f32,
                                            height: config.height as f32,
                                            stylesheet: Arc::new(sheet),
                                        }) {
                                            tracing::error!("Failed to send LayoutMsg::Compute for LoadUrl: {}", e);
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("Script Thread: Failed to load URL {}: {}", url, e);
                                }
                            }
                        }
                        ScriptMsg::Resize(w, h) => {
                            config.width = w;
                            config.height = h;
                            if let Some(ref root) = current_root {
                                if let Err(e) = layout_tx.send(LayoutMsg::Compute {
                                    root: NodePtr::clone_ptr(root),
                                    width: w as f32,
                                    height: h as f32,
                                    stylesheet: Arc::new(StyleSheet::default()), // Will fix this later
                                }) {
                                    tracing::error!("Failed to send LayoutMsg::Compute on Resize: {}", e);
                                }
                            }
                        }
                        ScriptMsg::DispatchEvent(node, event_type) => {
                            info!("Script Thread: Dispatching {} event to node", event_type);
                            // Here we could call JS dispatchEvent, but for now we just log
                            // In real DOM: node.dispatchEvent(new Event(event_type))
                            // To actually do this in boa:
                            let mut rust_event = xiaopeng_dom::event::Event::new(event_type.clone(), true, true);
                            xiaopeng_dom::Node::dispatch_event(&node, &mut rust_event);
                            
                            // Let's invoke JS:
                            let node_id = xiaopeng_script::bindings::dom::expose_node(NodePtr::clone_ptr(&node));
                            let script = format!("if (window.__dispatch_event) window.__dispatch_event({}, '{}');", node_id, event_type);
                            if let Err(e) = js_runtime.eval(&script) {
                                tracing::error!("Failed to eval __dispatch_event: {:?}", e);
                            }
                        }
                    }
                }
            }
        }
    }

    fn layout_loop(mut rx: UnboundedReceiver<LayoutMsg>, render_tx: UnboundedSender<RenderMsg>) {
        info!("Layout Thread started");
        let mut latest_layout_tree: Option<xiaopeng_layout::LayoutBox> = None;

        // We use blocking recv since this is a dedicated std::thread
        while let Some(msg) = rx.blocking_recv() {
            match msg {
                LayoutMsg::Compute { root, width, height, stylesheet } => {
                    info!("Layout Thread: Computing layout ({}x{})", width, height);
                    if let Ok(()) = xiaopeng_style::init_style() {
                        if let Ok(layout_root) = xiaopeng_layout::compute_layout(&root, width, height, &stylesheet) {
                            let display_list = xiaopeng_renderer::DisplayList::build(&layout_root);
                            if let Err(e) = render_tx.send(RenderMsg::Render(display_list)) {
                                tracing::error!("Failed to send RenderMsg: {}", e);
                            }
                            latest_layout_tree = Some(layout_root);
                        }
                    }
                }
                LayoutMsg::HitTest { x, y, event_type, script_tx } => {
                    if let Some(ref layout_root) = latest_layout_tree {
                        if let Some(hit_node) = xiaopeng_layout::hit_test(layout_root, x, y) {
                            info!("Layout Thread: Hit test found node for ({}, {})", x, y);
                            if let Err(e) = script_tx.send(ScriptMsg::DispatchEvent(hit_node, event_type)) {
                                tracing::error!("Failed to send ScriptMsg::DispatchEvent: {}", e);
                            }
                        } else {
                            info!("Layout Thread: Hit test found nothing for ({}, {})", x, y);
                        }
                    }
                }
            }
        }
    }

    fn render_loop(mut rx: UnboundedReceiver<RenderMsg>, config: crate::EngineConfig) {
        info!("Render Thread started");
        let font_manager = xiaopeng_renderer::font::FontManager::new();
        while let Some(msg) = rx.blocking_recv() {
            match msg {
                RenderMsg::Render(display_list) => {
                    info!("Render Thread: Rasterizing DisplayList");
                    if let Ok(canvas) = xiaopeng_renderer::render_display_list(
                        &display_list,
                        config.width,
                        config.height,
                        &font_manager,
                    ) {
                        if config.headless {
                            if let Some(path) = &config.headless_output {
                                info!("Exporting headless result to {}", path);
                                if path.ends_with(".ppm") {
                                    if let Err(e) = canvas.export_ppm(path) {
                                        tracing::error!("Failed to export PPM to {}: {:?}", path, e);
                                    }
                                } else {
                                    if let Err(e) = canvas.export_png(path) {
                                        tracing::error!("Failed to export PNG to {}: {:?}", path, e);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

async fn collect_stylesheets(root: &NodePtr, base_url: &str, net_client: &xiaopeng_net::NetClient) -> StyleSheet {
    let mut sheet = StyleSheet::default();
    
    // We can do a BFS or DFS on the DOM
    let mut stack = vec![NodePtr::clone_ptr(root)];
    while let Some(node) = stack.pop() {
        let node_ref = node.read().expect("Lock poisoned");
        if let xiaopeng_dom::NodeData::Element(el) = &node_ref.data {
            if el.tag_name == "style" {
                // Collect inner text
                let mut text = String::new();
                for child in &node_ref.children {
                    let c_ref = child.read().expect("Lock poisoned");
                    if let xiaopeng_dom::NodeData::Text(t) = &c_ref.data {
                        text.push_str(t);
                    }
                }
                let mut parser = xiaopeng_style::parser::CssParser::new(&text);
                let parsed = parser.parse();
                sheet.rules.extend(parsed.rules);
            } else if el.tag_name == "link" {
                let rel = el.attributes.get_named_item("rel").map(|a| a.value.clone()).unwrap_or_default();
                if rel == "stylesheet" {
                    if let Some(href) = el.attributes.get_named_item("href").map(|a| a.value.clone()) {
                        // resolve href relative to base_url
                        let absolute_url = if href.starts_with("http") { href } else { format!("{}/{}", base_url.trim_end_matches('/'), href.trim_start_matches('/')) };
                        if let Ok(res) = net_client.fetch(xiaopeng_net::Request::new("GET", &absolute_url)).await {
                            let css_text = String::from_utf8_lossy(&res.body).to_string();
                            let mut parser = xiaopeng_style::parser::CssParser::new(&css_text);
                            let parsed = parser.parse();
                            sheet.rules.extend(parsed.rules);
                        }
                    }
                }
            }
        }
        
        let mut children = node_ref.children.clone();
        children.reverse(); // to visit in source order (DFS)
        stack.extend(children);
    }
    
    sheet
}
