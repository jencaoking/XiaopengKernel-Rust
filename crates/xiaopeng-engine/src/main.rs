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
        headless: false,
        headless_output: None,
    };

    let mut engine = BrowserEngine::new(config);
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let url = &args[1];
        info!("Loading URL from command line: {}", url);
        engine.load_url(url).await?;
    } else {
        let default_url = "https://neverssl.com";
        info!("No URL provided, loading default: {}", default_url);
        engine.load_url(default_url).await?;
    }

    info!("🎉 Engine startup sequence completed successfully!");
    // Run in multi-threaded actor mode
    engine.run_actors()?;

    Ok(())
}
