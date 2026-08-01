//! DOM Event System

use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventPhase {
    None,
    CapturingPhase,
    AtTarget,
    BubblingPhase,
}

#[derive(Clone)]
pub struct Event {
    pub event_type: String,
    pub bubbles: bool,
    pub cancelable: bool,
    pub default_prevented: bool,
    pub propagation_stopped: bool,
    pub immediate_propagation_stopped: bool,
    pub phase: EventPhase,
}

impl Event {
    pub fn new(event_type: impl Into<String>, bubbles: bool, cancelable: bool) -> Self {
        Self {
            event_type: event_type.into(),
            bubbles,
            cancelable,
            default_prevented: false,
            propagation_stopped: false,
            immediate_propagation_stopped: false,
            phase: EventPhase::None,
        }
    }

    pub fn prevent_default(&mut self) {
        if self.cancelable {
            self.default_prevented = true;
        }
    }

    pub fn stop_propagation(&mut self) {
        self.propagation_stopped = true;
    }

    pub fn stop_immediate_propagation(&mut self) {
        self.propagation_stopped = true;
        self.immediate_propagation_stopped = true;
    }
}

pub trait EventListener: Send + Sync {
    fn handle_event(&self, event: &mut Event);
}

impl<F> EventListener for F
where
    F: Fn(&mut Event) + Send + Sync,
{
    fn handle_event(&self, event: &mut Event) {
        self(event)
    }
}

#[derive(Clone)]
pub struct EventListenerEntry {
    pub listener: Arc<dyn EventListener>,
    pub use_capture: bool,
}

impl std::fmt::Debug for EventListenerEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventListenerEntry")
         .field("use_capture", &self.use_capture)
         .field("listener", &"<function>")
         .finish()
    }
}
