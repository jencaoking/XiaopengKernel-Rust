//! XiaopengKernel DOM (Document Object Model) Module

pub mod event;
pub mod node;

pub use event::{Event, EventPhase};
pub use node::{ElementData, Node, NodePtr, NodeType};
use tracing::info;
use xiaopeng_common::XiaopengResult;

pub struct Document {
    pub root: NodePtr,
}

impl Document {
    pub fn new() -> Self {
        info!("Initializing DOM Document");
        Self {
            root: Node::new(NodeType::Document),
        }
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

pub fn init_dom() -> XiaopengResult<()> {
    info!("DOM module initialized");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_creation() {
        let doc = Document::new();
        assert!(matches!(doc.root.read().unwrap().node_type, NodeType::Document));
    }
}
