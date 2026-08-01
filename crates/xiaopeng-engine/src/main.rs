use tracing::info;
use xiaopeng_common::{init_logging, XiaopengResult};

#[tokio::main]
async fn main() -> XiaopengResult<()> {
    init_logging();
    info!("🚀 Launching XiaopengKernel (Rust Edition) v{}", env!("CARGO_PKG_VERSION"));

    let html_content = r#"
        <!DOCTYPE html>
        <html>
            <head><title>XiaopengKernel Rust</title></head>
            <body>
                <h1 style="color: red;">Hello XiaopengKernel Rust!</h1>
            </body>
        </html>
    "#;

    let _doc = xiaopeng_parser::parse_html(html_content)?;
    xiaopeng_style::init_style()?;
    xiaopeng_layout::compute_layout()?;
    xiaopeng_renderer::render_frame()?;

    info!("🎉 Engine startup sequence completed successfully!");
    Ok(())
}
