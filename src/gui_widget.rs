//! v72 retained widget tree primitives.

use std::collections::HashMap;

pub type WidgetId = u64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WidgetKind {
    Root,
    Row,
    Column,
    Panel,
    Label,
    Button,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WidgetProps {
    pub text: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WidgetNode {
    pub id: WidgetId,
    pub kind: WidgetKind,
    pub props: WidgetProps,
    pub children: Vec<WidgetId>,
}

impl WidgetNode {
    pub fn new(id: WidgetId, kind: WidgetKind) -> Self {
        Self {
            id,
            kind,
            props: WidgetProps::default(),
            children: Vec::new(),
        }
    }
}

#[derive(Debug, Default)]
pub struct WidgetTree {
    nodes: HashMap<WidgetId, WidgetNode>,
    pub root: WidgetId,
}

impl WidgetTree {
    pub fn new(root: WidgetId) -> Self {
        let mut nodes = HashMap::new();
        nodes.insert(root, WidgetNode::new(root, WidgetKind::Root));
        Self { nodes, root }
    }

    pub fn add_child(&mut self, parent: WidgetId, child: WidgetNode) -> bool {
        let child_id = child.id;
        if self.nodes.contains_key(&child_id) {
            return false;
        }
        if let Some(parent_node) = self.nodes.get_mut(&parent) {
            parent_node.children.push(child_id);
            self.nodes.insert(child_id, child);
            return true;
        }
        false
    }

    pub fn set_text(&mut self, id: WidgetId, text: impl Into<String>) -> bool {
        if let Some(node) = self.nodes.get_mut(&id) {
            node.props.text = Some(text.into());
            return true;
        }
        false
    }

    pub fn get(&self, id: WidgetId) -> Option<&WidgetNode> {
        self.nodes.get(&id)
    }

    pub fn iter_dfs(&self) -> Vec<WidgetId> {
        fn walk(tree: &WidgetTree, out: &mut Vec<WidgetId>, id: WidgetId) {
            out.push(id);
            if let Some(node) = tree.nodes.get(&id) {
                for child in &node.children {
                    walk(tree, out, *child);
                }
            }
        }

        let mut order = Vec::new();
        walk(self, &mut order, self.root);
        order
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_add_child_and_dfs_order() {
        let mut tree = WidgetTree::new(1);
        assert!(tree.add_child(1, WidgetNode::new(2, WidgetKind::Panel)));
        assert!(tree.add_child(2, WidgetNode::new(3, WidgetKind::Button)));
        assert!(tree.add_child(2, WidgetNode::new(4, WidgetKind::Label)));

        assert_eq!(tree.iter_dfs(), vec![1, 2, 3, 4]);
    }

    #[test]
    fn duplicate_id_rejected() {
        let mut tree = WidgetTree::new(1);
        assert!(tree.add_child(1, WidgetNode::new(2, WidgetKind::Panel)));
        assert!(!tree.add_child(1, WidgetNode::new(2, WidgetKind::Label)));
    }
}
