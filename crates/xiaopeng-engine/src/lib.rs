//! XiaopengKernel Core Engine Entry & API

pub mod browsing_context;
pub mod event_loop;

pub use browsing_context::BrowsingContext;
pub use event_loop::EventLoop;
use tracing::{info, instrument, warn};
use xiaopeng_common::XiaopengResult;
use xiaopeng_script::JsRuntime;

#[derive(Debug, Default, Clone)]
pub struct EngineConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
}

pub struct BrowserEngine {
    pub config: EngineConfig,
    pub context: BrowsingContext,
    pub event_loop: EventLoop,
    pub js_runtime: JsRuntime,
}

impl BrowserEngine {
    pub fn new(config: EngineConfig) -> Self {
        info!(?config, "Initializing BrowserEngine");
        let js_runtime = JsRuntime::new()
            .unwrap_or_else(|e| {
                warn!("Failed to create JsRuntime: {e}; falling back to default");
                JsRuntime::default()
            });
        Self {
            config,
            context: BrowsingContext::new(),
            event_loop: EventLoop::new(),
            js_runtime,
        }
    }

    #[instrument(skip(self, html_input), fields(input_bytes = html_input.len()))]
    pub fn load_html(&mut self, html_input: &str) -> XiaopengResult<()> {
        info!("BrowserEngine: Triggering HTML document loading pipeline");
        let doc = xiaopeng_parser::parse_html(html_input)?;

        // --- Expose DOM to JS Engine ---
        let root_id = xiaopeng_script::bindings::dom::expose_node(std::sync::Arc::clone(&doc.root));
        let init_script = format!("____init_document({});", root_id);
        if let Err(e) = self.js_runtime.eval(&init_script) {
            tracing::warn!("Failed to init JS Document bridge: {}", e);
        }

        // --- Execute inline <script> tags ---
        self.run_inline_scripts(&doc.root);

        self.context.document = Some(doc);

        xiaopeng_style::init_style()?;

        let mut display_list = xiaopeng_renderer::DisplayList::new();
        if let Some(ref doc) = self.context.document {
            let layout_root = xiaopeng_layout::compute_layout(
                &doc.root,
                self.config.width as f32,
                self.config.height as f32,
            )?;
            display_list = xiaopeng_renderer::DisplayList::build(&layout_root);
        }

        let _canvas = xiaopeng_renderer::render_display_list(
            &display_list,
            self.config.width,
            self.config.height,
        )?;

        info!("BrowserEngine: Pipeline processing complete");
        Ok(())
    }

    /// Walk the DOM tree and execute the text content of every `<script>` element.
    fn run_inline_scripts(&mut self, node: &xiaopeng_dom::NodePtr) {
        use xiaopeng_dom::NodeData;
        let children = {
            let n = node.read().unwrap();
            match &n.data {
                NodeData::Element(el) if el.tag_name == "script" => {
                    // Collect text content of this script node
                    let mut src = String::new();
                    for child in &n.children {
                        if let NodeData::Text(t) = &child.read().unwrap().data {
                            src.push_str(t);
                        }
                    }
                    if !src.trim().is_empty() {
                        info!("Running inline <script> ({} bytes)", src.len());
                        if let Err(e) = self.js_runtime.eval(&src) {
                            warn!("Inline script error: {e}");
                        }
                    }
                    return;
                }
                _ => n.children.clone(),
            }
        };
        for child in children {
            self.run_inline_scripts(&child);
        }
    }
}
