use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};
use tracing::{info, warn};

use super::{EngineActors, ScriptMsg};

pub struct ConstellationApp {
    pub config: crate::EngineConfig,
    pub actors: EngineActors,
    pub window: Option<Arc<Window>>,
    pub render_rx: Option<tokio::sync::mpsc::UnboundedReceiver<super::RenderMsg>>,
    pub renderer: Option<xiaopeng_renderer::WgpuRenderer>,
    pub latest_display_list: Option<xiaopeng_renderer::DisplayList>,
}

impl ConstellationApp {
    pub fn new(config: crate::EngineConfig, initial_doc: Option<xiaopeng_dom::Document>) -> Self {
        let (actors, render_rx) = EngineActors::spawn(config.clone(), initial_doc);
        Self {
            actors,
            config,
            window: None,
            render_rx,
            renderer: None,
            latest_display_list: None,
        }
    }
}

impl ApplicationHandler for ConstellationApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.config.headless {
            info!("Constellation: Headless mode, running actors and waiting for export...");
            std::thread::sleep(std::time::Duration::from_millis(500));
            info!("Constellation: Exiting event loop.");
            event_loop.exit();
            return;
        }

        let window_attributes = Window::default_attributes()
            .with_title("XiaopengKernel (Actor Mode)")
            .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0));
            
        match event_loop.create_window(window_attributes) {
            Ok(window) => {
                info!("Constellation: UI Window created successfully");
                let window = Arc::new(window);
                self.window = Some(window.clone());
                
                // Initialize WgpuRenderer
                if let Ok(renderer) = pollster::block_on(xiaopeng_renderer::WgpuRenderer::new(window.clone())) {
                    info!("Constellation: WgpuRenderer initialized successfully");
                    self.renderer = Some(renderer);
                } else {
                    warn!("Constellation: Failed to initialize WgpuRenderer");
                }
                
                window.request_redraw();
            }
            Err(e) => {
                warn!("Constellation: Failed to create window: {:?}", e);
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                info!("Constellation: Close requested, shutting down Actor system.");
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                info!("Constellation: Resized to {}x{}", size.width, size.height);
                let _ = self.actors.script_tx.send(ScriptMsg::Resize(size.width, size.height));
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                // Check if a new DisplayList has arrived
                if let Some(rx) = &mut self.render_rx {
                    while let Ok(msg) = rx.try_recv() {
                        let super::RenderMsg::Render(dl) = msg;
                        self.latest_display_list = Some(dl);
                    }
                }
                
                // Render the latest display list
                if let (Some(renderer), Some(dl)) = (&mut self.renderer, &self.latest_display_list) {
                    if let Err(e) = renderer.render(dl) {
                        warn!("Constellation: Render error: {}", e);
                    }
                }
                
                // Continue animation loop
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            // other events can be forwarded here...
            _ => (),
        }
    }
}
