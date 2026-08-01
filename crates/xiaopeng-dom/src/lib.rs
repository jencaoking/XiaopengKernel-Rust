//! XiaopengKernel DOM (Document Object Model) Module

pub mod event;
pub mod node;

pub use event::{Event, EventPhase, EventListener};
pub use node::{ElementData, Node, NodeData, NodePtr, NodeType};
use tracing::info;
use xiaopeng_common::XiaopengResult;

pub struct Document {
    pub root: NodePtr,
}

impl Document {
    pub fn new() -> Self {
        info!("Initializing DOM Document");
        Self {
            root: Node::new(NodeData::Document),
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
        assert!(matches!(
            doc.root.read().unwrap().data,
            NodeData::Document
        ));
    }
}
