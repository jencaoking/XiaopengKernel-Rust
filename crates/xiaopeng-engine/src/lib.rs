//! XiaopengKernel Core Engine Entry & API

pub mod browsing_context;

pub use browsing_context::BrowsingContext;
use tracing::{info, instrument};
use xiaopeng_common::XiaopengResult;

#[derive(Debug, Default, Clone)]
pub struct EngineConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
}

pub struct BrowserEngine {
    pub config: EngineConfig,
    pub context: BrowsingContext,
}

impl BrowserEngine {
    pub fn new(config: EngineConfig) -> Self {
        info!(?config, "Initializing BrowserEngine");
        Self {
            config,
            context: BrowsingContext::new(),
        }
    }

    #[instrument(skip(self, html_input), fields(input_bytes = html_input.len()))]
    pub fn load_html(&mut self, html_input: &str) -> XiaopengResult<()> {
        info!("BrowserEngine: Triggering HTML document loading pipeline");
        let doc = xiaopeng_parser::parse_html(html_input)?;
        self.context.document = Some(doc);

        xiaopeng_style::init_style()?;
        
        let mut display_list = xiaopeng_renderer::DisplayList::new();
        if let Some(ref doc) = self.context.document {
            let layout_root = xiaopeng_layout::compute_layout(&doc.root, self.config.width as f32, self.config.height as f32)?;
            display_list = xiaopeng_renderer::DisplayList::build(&layout_root);
        }
        
        let _canvas = xiaopeng_renderer::render_display_list(&display_list, self.config.width, self.config.height)?;

        info!("BrowserEngine: Pipeline processing complete");
        Ok(())
    }
}
