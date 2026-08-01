//! Browsing Context & Frame management

use xiaopeng_dom::Document;
use tracing::info;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageLifecycleState {
    Active,
    Passive,
    Hidden,
    Frozen,
    Terminated,
    Discarded,
}

pub struct BrowsingContext {
    pub document: Option<Document>,
    pub lifecycle_state: PageLifecycleState,
}

impl BrowsingContext {
    pub fn new() -> Self {
        Self { 
            document: None,
            lifecycle_state: PageLifecycleState::Active,
        }
    }

    pub fn transition_lifecycle(&mut self, new_state: PageLifecycleState) {
        if self.lifecycle_state == new_state {
            return;
        }
        
        info!(
            "Page Lifecycle Transition: {:?} -> {:?}", 
            self.lifecycle_state, new_state
        );
        
        // Typical W3C Page Lifecycle event dispatching logic would go here.
        // For example:
        // - Active -> Hidden: dispatch 'visibilitychange'
        // - Hidden -> Frozen: dispatch 'freeze'
        // - Active -> Terminated: dispatch 'pagehide' -> 'visibilitychange' -> 'unload'
        
        self.lifecycle_state = new_state;
    }
}

impl Default for BrowsingContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lifecycle_transitions() {
        let mut context = BrowsingContext::new();
        assert_eq!(context.lifecycle_state, PageLifecycleState::Active);

        context.transition_lifecycle(PageLifecycleState::Hidden);
        assert_eq!(context.lifecycle_state, PageLifecycleState::Hidden);

        context.transition_lifecycle(PageLifecycleState::Frozen);
        assert_eq!(context.lifecycle_state, PageLifecycleState::Frozen);

        context.transition_lifecycle(PageLifecycleState::Terminated);
        assert_eq!(context.lifecycle_state, PageLifecycleState::Terminated);
    }
}
