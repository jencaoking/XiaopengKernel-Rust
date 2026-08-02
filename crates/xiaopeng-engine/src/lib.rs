//! XiaopengKernel Core Engine Entry & API

pub mod browsing_context;
pub mod event_loop;

pub use browsing_context::BrowsingContext;
pub use event_loop::EventLoop;
use tracing::{info, instrument, warn};
use xiaopeng_common::XiaopengResult;
use xiaopeng_script::JsRuntime;
use xiaopeng_parser::html::{HtmlTokenizer, HtmlTreeBuilder, HtmlToken};

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
        self.finish_loading(doc)?;
        Ok(())
    }

    #[instrument(skip(self, url))]
    pub async fn load_url(&mut self, url: &str) -> XiaopengResult<()> {
        let url_clone = url.to_string();
        info!("BrowserEngine: Triggering background streaming load pipeline for {}", url_clone);
        
        let doc = tokio::spawn(async move {
            let resp = xiaopeng_net::fetch_stream(&url_clone).await?;
            let mut rx = resp.body_stream;

            let mut tokenizer = HtmlTokenizer::new();
            let mut tree_builder = HtmlTreeBuilder::new();

            while let Some(chunk_res) = rx.recv().await {
                let chunk = chunk_res?;
                let chunk_str = String::from_utf8_lossy(&chunk);
                tokenizer.push_chunk(&chunk_str);

                while let Ok(Some(token)) = tokenizer.next_token() {
                    tree_builder.process_token(token);
                }
            }

            tokenizer.end_of_file();
            while let Ok(Some(token)) = tokenizer.next_token() {
                let is_eof = token == HtmlToken::Eof;
                tree_builder.process_token(token);
                if is_eof {
                    break;
                }
            }
            Ok::<_, xiaopeng_common::XiaopengError>(tree_builder.document)
        })
        .await
        .map_err(|e| xiaopeng_common::XiaopengError::NetworkError {
            url: url.to_string(),
            message: format!("Background parsing task panicked: {e}"),
        })??;

        self.finish_loading(doc)?;
        Ok(())
    }

    fn finish_loading(&mut self, doc: xiaopeng_dom::Document) -> XiaopengResult<()> {
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
