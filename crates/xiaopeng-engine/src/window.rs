use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::{WindowEvent, ElementState, MouseButton};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};
use tracing::{info, warn};
use crate::BrowserEngine;

pub struct BrowserApp {
    pub engine: BrowserEngine,
    pub window: Option<Arc<Window>>,
}

impl BrowserApp {
    pub fn new(engine: BrowserEngine) -> Self {
        Self { engine, window: None }
    }
}

impl ApplicationHandler for BrowserApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_title(self.engine.config.title.clone())
            .with_inner_size(winit::dpi::LogicalSize::new(
                self.engine.config.width as f64,
                self.engine.config.height as f64,
            ));
            
        match event_loop.create_window(window_attributes) {
            Ok(window) => {
                info!("Window created successfully");
                self.window = Some(Arc::new(window));
                
                if let Some(win) = &self.window {
                    win.request_redraw();
                }
            }
            Err(e) => {
                warn!("Failed to create window: {:?}", e);
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
                info!("Close requested, exiting.");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                // We integrate with EventLoop macro/micro tasks here for now
                // In the future this should call WGPU rendering
                self.engine.event_loop.step(&mut || {
                    // Render callback
                });
                
                if let Some(window) = &self.window {
                    // Continue animation loop
                    window.request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                // Dispatch mouse move to DOM
                if let Some(doc) = &self.engine.context.document {
                    let mut js_event = xiaopeng_dom::Event::new("mousemove".to_string(), true, true);
                    xiaopeng_dom::Node::dispatch_event(&doc.root, &mut js_event);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if state == ElementState::Pressed && button == MouseButton::Left {
                    info!("Mouse left click");
                    if let Some(doc) = &self.engine.context.document {
                        let mut js_event = xiaopeng_dom::Event::new("click".to_string(), true, true);
                        xiaopeng_dom::Node::dispatch_event(&doc.root, &mut js_event);
                    }
                }
            }
            WindowEvent::Resized(size) => {
                info!("Resized to {}x{}", size.width, size.height);
                self.engine.config.width = size.width;
                self.engine.config.height = size.height;
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => (),
        }
    }
}
