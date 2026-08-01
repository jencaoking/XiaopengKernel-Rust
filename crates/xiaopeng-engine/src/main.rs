use tracing::info;
use xiaopeng_common::{init_logging, XiaopengResult};
use xiaopeng_engine::{BrowserEngine, EngineConfig};

#[tokio::main]
async fn main() -> XiaopengResult<()> {
    init_logging();
    info!(
        "🚀 Launching XiaopengKernel (Rust Edition) v{}",
        env!("CARGO_PKG_VERSION")
    );

    let config = EngineConfig {
        title: "XiaopengKernel Rust Demo".into(),
        width: 800,
        height: 600,
    };

    let mut engine = BrowserEngine::new(config);
    engine.load_html(
        r#"
        <!DOCTYPE html>
        <html>
            <head><title>XiaopengKernel Rust</title></head>
            <body>
                <h1 style="color: red;">Hello XiaopengKernel Rust!</h1>
            </body>
        </html>
    "#,
    )?;

    info!("🎉 Engine startup sequence completed successfully!");
    Ok(())
}
