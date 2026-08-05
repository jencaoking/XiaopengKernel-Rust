use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tracing::info;
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
    DispatchEvent(NodePtr, String),
}

/// Messages sent to the Layout Thread
#[derive(Debug)]
pub enum LayoutMsg {
    Compute {
        root: NodePtr,
        width: f32,
        height: f32,
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
            let _ = layout_tx.send(LayoutMsg::Compute {
                root: doc.root,
                width: config.width as f32,
                height: config.height as f32,
            });
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
                .unwrap();
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
        
        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    if xiaopeng_dom::node::take_dom_dirty() {
                        if let Some(ref root) = current_root {
                            info!("Script Thread: DOM mutation detected, triggering incremental layout");
                            let _ = layout_tx.send(LayoutMsg::Compute {
                                root: Arc::clone(root),
                                width: config.width as f32,
                                height: config.height as f32,
                            });
                        }
                    }
                }
                msg_opt = rx.recv() => {
                    let Some(msg) = msg_opt else { break; };
                    match msg {
                        ScriptMsg::LoadHtml(html) => {
                            info!("Script Thread: Parsing HTML");
                            if let Ok(doc) = xiaopeng_parser::parse_html(&html) {
                                // init JS
                                let root_id = xiaopeng_script::bindings::dom::expose_node(Arc::clone(&doc.root));
                                let _ = js_runtime.eval(&format!("____init_document({});", root_id));
                                
                                current_root = Some(Arc::clone(&doc.root));
                                
                                // Send to layout
                                let _ = layout_tx.send(LayoutMsg::Compute {
                                    root: doc.root,
                                    width: config.width as f32,
                                    height: config.height as f32,
                                });
                            }
                        }
                        ScriptMsg::LoadUrl(url) => {
                            info!("Script Thread: Loading URL {}", url);
                        }
                        ScriptMsg::Resize(w, h) => {
                            config.width = w;
                            config.height = h;
                            if let Some(ref root) = current_root {
                                let _ = layout_tx.send(LayoutMsg::Compute {
                                    root: Arc::clone(root),
                                    width: w as f32,
                                    height: h as f32,
                                });
                            }
                        }
                        ScriptMsg::DispatchEvent(node, event_type) => {
                            info!("Script Thread: Dispatching {} event to node", event_type);
                            // Here we could call JS dispatchEvent, but for now we just log
                            // In real DOM: node.dispatchEvent(new Event(event_type))
                            // To actually do this in boa:
                            let mut js_event = xiaopeng_dom::event::Event::new(event_type.clone(), true, true);
                            
                            // Note: full event bubbling/capturing logic should be in xiaopeng-script bindings
                            // We trigger the native rust listeners for now:
                            let node_read = node.read().unwrap();
                            if let Some(listeners) = node_read.listeners.get(&event_type) {
                                for entry in listeners {
                                    // Trigger callback
                                    // (Real implementation invokes JS callback via js_runtime)
                                }
                            }
                            // Let's invoke JS:
                            let node_id = xiaopeng_script::bindings::dom::expose_node(Arc::clone(&node));
                            let script = format!("if (window.__dispatch_event) window.__dispatch_event({}, '{}');", node_id, event_type);
                            let _ = js_runtime.eval(&script);
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
                LayoutMsg::Compute { root, width, height } => {
                    info!("Layout Thread: Computing layout ({}x{})", width, height);
                    if let Ok(()) = xiaopeng_style::init_style() {
                        if let Ok(layout_root) = xiaopeng_layout::compute_layout(&root, width, height) {
                            let display_list = xiaopeng_renderer::DisplayList::build(&layout_root);
                            let _ = render_tx.send(RenderMsg::Render(display_list));
                            latest_layout_tree = Some(layout_root);
                        }
                    }
                }
                LayoutMsg::HitTest { x, y, event_type, script_tx } => {
                    if let Some(ref layout_root) = latest_layout_tree {
                        if let Some(hit_node) = xiaopeng_layout::hit_test(layout_root, x, y) {
                            info!("Layout Thread: Hit test found node for ({}, {})", x, y);
                            let _ = script_tx.send(ScriptMsg::DispatchEvent(hit_node, event_type));
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
