//! v72 deterministic layout snapshots (flex/grid subset).

use crate::gui_widget::{WidgetId, WidgetKind, WidgetTree};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutBox {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutMode {
    FlexRow,
    Grid2,
}

pub fn compute_layout(
    tree: &WidgetTree,
    root_width: i32,
    row_height: i32,
    mode: LayoutMode,
) -> HashMap<WidgetId, LayoutBox> {
    let mut out = HashMap::new();
    let order = tree.iter_dfs();
    if order.is_empty() {
        return out;
    }

    let root = order[0];
    out.insert(
        root,
        LayoutBox {
            x: 0,
            y: 0,
            w: root_width,
            h: row_height,
        },
    );

    match mode {
        LayoutMode::FlexRow => {
            let mut cursor_x = 0;
            for id in order.iter().skip(1) {
                let is_container = matches!(
                    tree.get(*id).map(|n| &n.kind),
                    Some(WidgetKind::Row) | Some(WidgetKind::Column) | Some(WidgetKind::Panel)
                );
                let width = if is_container { root_width / 2 } else { root_width / 4 };
                out.insert(
                    *id,
                    LayoutBox {
                        x: cursor_x,
                        y: row_height,
                        w: width,
                        h: row_height,
                    },
                );
                cursor_x += width;
            }
        }
        LayoutMode::Grid2 => {
            for (idx, id) in order.iter().skip(1).enumerate() {
                let col = (idx % 2) as i32;
                let row = (idx / 2) as i32;
                out.insert(
                    *id,
                    LayoutBox {
                        x: col * (root_width / 2),
                        y: (row + 1) * row_height,
                        w: root_width / 2,
                        h: row_height,
                    },
                );
            }
        }
    }

    out
}

pub fn snapshot(layout: &HashMap<WidgetId, LayoutBox>) -> String {
    let mut items: Vec<_> = layout.iter().collect();
    items.sort_by_key(|(id, _)| **id);
    items
        .into_iter()
        .map(|(id, b)| format!("{id}:{}:{}:{}:{}", b.x, b.y, b.w, b.h))
        .collect::<Vec<_>>()
        .join("|")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui_widget::{WidgetNode, WidgetTree};

    #[test]
    fn flex_layout_snapshot_is_deterministic() {
        let mut tree = WidgetTree::new(1);
        assert!(tree.add_child(1, WidgetNode::new(2, WidgetKind::Panel)));
        assert!(tree.add_child(1, WidgetNode::new(3, WidgetKind::Button)));

        let a = compute_layout(&tree, 400, 40, LayoutMode::FlexRow);
        let b = compute_layout(&tree, 400, 40, LayoutMode::FlexRow);

        assert_eq!(snapshot(&a), snapshot(&b));
        assert_eq!(snapshot(&a), "1:0:0:400:40|2:0:40:200:40|3:200:40:100:40");
    }

    #[test]
    fn grid_layout_positions_two_columns() {
        let mut tree = WidgetTree::new(1);
        assert!(tree.add_child(1, WidgetNode::new(2, WidgetKind::Button)));
        assert!(tree.add_child(1, WidgetNode::new(3, WidgetKind::Button)));
        assert!(tree.add_child(1, WidgetNode::new(4, WidgetKind::Button)));

        let g = compute_layout(&tree, 300, 30, LayoutMode::Grid2);
        assert_eq!(snapshot(&g), "1:0:0:300:30|2:0:30:150:30|3:150:30:150:30|4:0:60:150:30");
    }
}
