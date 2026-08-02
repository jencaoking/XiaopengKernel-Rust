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

        // --- Execute inline <script> content (extracted from raw HTML before parsing)
        // NOTE: The tokenizer does not yet implement ScriptData state, so script text
        // content is not reliably preserved in the DOM tree.  We extract it directly
        // from the source string instead.
        self.run_scripts_from_html(html_input);

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

    /// Extract and execute every `<script>...</script>` block found in the raw HTML source.
    /// This works around the current tokenizer's lack of a proper ScriptData state,
    /// which causes script text to be lost during DOM construction.
    fn run_scripts_from_html(&mut self, html: &str) {
        let html_lower = html.to_lowercase();
        let mut search_from = 0usize;

        loop {
            // Find next <script opening tag
            let script_open = match html_lower[search_from..].find("<script") {
                Some(pos) => search_from + pos,
                None => break,
            };

            // Advance past the opening tag's `>`
            let tag_close = match html[script_open..].find('>') {
                Some(pos) => script_open + pos + 1,
                None => break,
            };

            // Skip `<script src=...>` external scripts (no inline content to run)
            let opening_tag = &html[script_open..tag_close];
            if opening_tag.to_lowercase().contains("src=") {
                search_from = tag_close;
                continue;
            }

            // Find the matching </script>
            let script_end = match html_lower[tag_close..].find("</script") {
                Some(pos) => tag_close + pos,
                None => break,
            };

            let content = html[tag_close..script_end].trim();
            if !content.is_empty() {
                info!("Running inline <script> ({} bytes)", content.len());
                if let Err(e) = self.js_runtime.eval(content) {
                    warn!("Inline script error: {e}");
                }
            }

            // Continue searching after </script>
            search_from = script_end + "</script>".len();
        }
    }
}
