//! v73 input/event routing with capture+bubble and focus management.

use crate::gui_widget::{WidgetId, WidgetTree};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputEvent {
    KeyDown { key: String },
    MouseDown { x: i32, y: i32 },
    MouseUp { x: i32, y: i32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Capture,
    Bubble,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutedEvent {
    pub phase: Phase,
    pub widget: WidgetId,
}

#[derive(Debug, Default)]
pub struct FocusManager {
    focused: Option<WidgetId>,
    shortcuts: HashMap<String, WidgetId>,
}

impl FocusManager {
    pub fn set_focus(&mut self, id: WidgetId) {
        self.focused = Some(id);
    }

    pub fn focused(&self) -> Option<WidgetId> {
        self.focused
    }

    pub fn register_shortcut(&mut self, chord: impl Into<String>, target: WidgetId) -> bool {
        let chord = chord.into();
        if self.shortcuts.contains_key(&chord) {
            return false;
        }
        self.shortcuts.insert(chord, target);
        true
    }

    pub fn resolve_shortcut(&self, chord: &str) -> Option<WidgetId> {
        self.shortcuts.get(chord).copied()
    }

    pub fn has_conflict(&self, chord: &str) -> bool {
        self.shortcuts.contains_key(chord)
    }
}

pub fn route_event(
    tree: &WidgetTree,
    target: WidgetId,
    _event: &InputEvent,
) -> Vec<RoutedEvent> {
    let dfs = tree.iter_dfs();
    let mut parent_of: HashMap<WidgetId, WidgetId> = HashMap::new();
    let set: HashSet<WidgetId> = dfs.into_iter().collect();

    for id in &set {
        if let Some(node) = tree.get(*id) {
            for child in &node.children {
                parent_of.insert(*child, *id);
            }
        }
    }

    if !set.contains(&target) {
        return Vec::new();
    }

    let mut path = vec![target];
    let mut cursor = target;
    while let Some(parent) = parent_of.get(&cursor) {
        path.push(*parent);
        cursor = *parent;
    }
    path.reverse();

    let mut out = Vec::with_capacity(path.len() * 2);
    for id in &path {
        out.push(RoutedEvent {
            phase: Phase::Capture,
            widget: *id,
        });
    }
    for id in path.into_iter().rev() {
        out.push(RoutedEvent {
            phase: Phase::Bubble,
            widget: id,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui_widget::{WidgetKind, WidgetNode, WidgetTree};

    #[test]
    fn route_uses_capture_then_bubble() {
        let mut tree = WidgetTree::new(1);
        assert!(tree.add_child(1, WidgetNode::new(2, WidgetKind::Panel)));
        assert!(tree.add_child(2, WidgetNode::new(3, WidgetKind::Button)));

        let routed = route_event(&tree, 3, &InputEvent::MouseDown { x: 10, y: 10 });
        let phases: Vec<_> = routed.iter().map(|r| (r.phase, r.widget)).collect();
        assert_eq!(
            phases,
            vec![
                (Phase::Capture, 1),
                (Phase::Capture, 2),
                (Phase::Capture, 3),
                (Phase::Bubble, 3),
                (Phase::Bubble, 2),
                (Phase::Bubble, 1),
            ]
        );
    }

    #[test]
    fn focus_manager_detects_shortcut_conflicts() {
        let mut fm = FocusManager::default();
        assert!(fm.register_shortcut("Ctrl+S", 10));
        assert!(fm.has_conflict("Ctrl+S"));
        assert!(!fm.register_shortcut("Ctrl+S", 11));
        assert_eq!(fm.resolve_shortcut("Ctrl+S"), Some(10));
    }
}
