use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::{WindowEvent, ElementState, MouseButton};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};
use tracing::{info, warn};

use super::{EngineActors, ScriptMsg};

pub struct ConstellationApp {
    pub actors: EngineActors,
    pub window: Option<Arc<Window>>,
}

impl ConstellationApp {
    pub fn new(config: crate::EngineConfig) -> Self {
        Self {
            actors: EngineActors::spawn(config),
            window: None,
        }
    }
}

impl ApplicationHandler for ConstellationApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_title("XiaopengKernel (Actor Mode)")
            .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0));
            
        match event_loop.create_window(window_attributes) {
            Ok(window) => {
                info!("Constellation: UI Window created successfully");
                self.window = Some(Arc::new(window));
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
            }
            // other events can be forwarded here...
            _ => (),
        }
    }
}
