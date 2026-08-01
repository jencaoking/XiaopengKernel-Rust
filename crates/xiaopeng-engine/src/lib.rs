//! XiaopengKernel Core Engine Entry & API

pub mod browsing_context;

pub use browsing_context::BrowsingContext;
use tracing::info;
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
        Self {
            config,
            context: BrowsingContext::new(),
        }
    }

    pub fn load_html(&mut self, html_input: &str) -> XiaopengResult<()> {
        info!("BrowserEngine: Loading HTML string");
        let doc = xiaopeng_parser::parse_html(html_input)?;
        self.context.document = Some(doc);

        xiaopeng_style::init_style()?;
        xiaopeng_layout::compute_layout()?;
        xiaopeng_renderer::render_frame()?;

        Ok(())
    }
}
