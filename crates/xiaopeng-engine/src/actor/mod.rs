use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tracing::{info, warn};
use std::sync::Arc;

pub mod app;
pub use app::ConstellationApp;

use xiaopeng_dom::NodePtr;
use xiaopeng_renderer::DisplayList;

/// Messages sent to the Script Thread (DOM + JS)
#[derive(Debug)]
pub enum ScriptMsg {
    LoadHtml(String),
    LoadUrl(String),
    Resize(u32, u32),
    // Event(xiaopeng_dom::Event),
}

/// Messages sent to the Layout Thread
#[derive(Debug)]
pub enum LayoutMsg {
    Compute {
        root: NodePtr,
        width: f32,
        height: f32,
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
    pub fn spawn(config: crate::EngineConfig) -> Self {
        let (script_tx, script_rx) = unbounded_channel();
        let (layout_tx, layout_rx) = unbounded_channel();
        let (render_tx, render_rx) = unbounded_channel();

        // Spawn Layout Thread
        let render_tx_clone = render_tx.clone();
        std::thread::spawn(move || {
            Self::layout_loop(layout_rx, render_tx_clone);
        });

        // Spawn Render Thread
        let config_clone = config.clone();
        std::thread::spawn(move || {
            Self::render_loop(render_rx, config_clone);
        });

        // Spawn Script Thread (Runs within a Tokio local set or block_on)
        let layout_tx_clone = layout_tx.clone();
        let config_clone2 = config.clone();
        std::thread::spawn(move || {
            // Script thread typically requires a Tokio runtime for network fetch
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                Self::script_loop(script_rx, layout_tx_clone, config_clone2).await;
            });
        });

        Self {
            script_tx,
            layout_tx,
            render_tx,
        }
    }

    async fn script_loop(
        mut rx: UnboundedReceiver<ScriptMsg>,
        layout_tx: UnboundedSender<LayoutMsg>,
        mut config: crate::EngineConfig,
    ) {
        info!("Script Thread started");
        let mut js_runtime = xiaopeng_script::JsRuntime::new().unwrap_or_default();
        
        while let Some(msg) = rx.recv().await {
            match msg {
                ScriptMsg::LoadHtml(html) => {
                    info!("Script Thread: Parsing HTML");
                    if let Ok(doc) = xiaopeng_parser::parse_html(&html) {
                        // init JS
                        let root_id = xiaopeng_script::bindings::dom::expose_node(Arc::clone(&doc.root));
                        let _ = js_runtime.eval(&format!("____init_document({});", root_id));
                        
                        // Send to layout
                        let _ = layout_tx.send(LayoutMsg::Compute {
                            root: doc.root,
                            width: config.width as f32,
                            height: config.height as f32,
                        });
                    }
                }
                ScriptMsg::LoadUrl(url) => {
                    // Similar to load_url in Engine
                    info!("Script Thread: Loading URL {}", url);
                }
                ScriptMsg::Resize(w, h) => {
                    config.width = w;
                    config.height = h;
                    // Trigger a re-layout here if document is retained
                }
            }
        }
    }

    fn layout_loop(mut rx: UnboundedReceiver<LayoutMsg>, render_tx: UnboundedSender<RenderMsg>) {
        info!("Layout Thread started");
        // We use blocking recv since this is a dedicated std::thread
        while let Some(msg) = rx.blocking_recv() {
            match msg {
                LayoutMsg::Compute { root, width, height } => {
                    info!("Layout Thread: Computing layout ({}x{})", width, height);
                    if let Ok(()) = xiaopeng_style::init_style() {
                        if let Ok(layout_root) = xiaopeng_layout::compute_layout(&root, width, height) {
                            let display_list = xiaopeng_renderer::DisplayList::build(&layout_root);
                            let _ = render_tx.send(RenderMsg::Render(display_list));
                        }
                    }
                }
            }
        }
    }

    fn render_loop(mut rx: UnboundedReceiver<RenderMsg>, config: crate::EngineConfig) {
        info!("Render Thread started");
        while let Some(msg) = rx.blocking_recv() {
            match msg {
                RenderMsg::Render(display_list) => {
                    info!("Render Thread: Rasterizing DisplayList");
                    if let Ok(canvas) = xiaopeng_renderer::render_display_list(
                        &display_list,
                        config.width,
                        config.height,
                    ) {
                        if config.headless {
                            if let Some(path) = &config.headless_output {
                                info!("Exporting headless result to {}", path);
                                if path.ends_with(".ppm") {
                                    let _ = canvas.export_ppm(path);
                                } else {
                                    let _ = canvas.export_png(path);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
