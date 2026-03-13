use crate::app::Tab;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDir {
    Horizontal, // side by side: left | right
    Vertical,   // stacked: top / bottom
}

#[derive(Debug, Clone)]
pub enum Tile {
    Leaf(Tab),
    Split {
        dir: SplitDir,
        ratio: f32,
        left: Box<Tile>,
        right: Box<Tile>,
    },
}

/// A path from root to a specific leaf (sequence of Left/Right turns).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path(pub Vec<Side>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

#[derive(Debug, Clone)]
pub struct TileLayout {
    pub root: Tile,
    pub focus_path: Path,
}

impl TileLayout {
    pub fn new(tab: Tab) -> Self {
        TileLayout {
            root: Tile::Leaf(tab),
            focus_path: Path(vec![]),
        }
    }

    pub fn focused_tab(&self) -> Tab {
        get_leaf_tab(&self.root, &self.focus_path.0).expect("focus path must point to a leaf")
    }

    pub fn visible_tabs(&self) -> Vec<Tab> {
        let mut tabs = vec![];
        collect_tabs(&self.root, &mut tabs);
        tabs
    }

    pub fn is_single(&self) -> bool {
        matches!(self.root, Tile::Leaf(_))
    }

    pub fn focus_next(&mut self) {
        let paths = self.all_paths();
        if let Some(idx) = paths.iter().position(|p| p == &self.focus_path) {
            self.focus_path = paths[(idx + 1) % paths.len()].clone();
        }
    }

    pub fn focus_prev(&mut self) {
        let paths = self.all_paths();
        if let Some(idx) = paths.iter().position(|p| p == &self.focus_path) {
            let n = paths.len();
            self.focus_path = paths[(idx + n - 1) % n].clone();
        }
    }

    /// Split the focused pane. The new pane starts with the same tab.
    pub fn split(&mut self, dir: SplitDir) {
        let current_tab = self.focused_tab();
        let path = self.focus_path.0.clone();
        split_at(&mut self.root, &path, dir, current_tab);
        // Move focus to the new right/bottom pane
        let mut new_path = self.focus_path.0.clone();
        new_path.push(Side::Right);
        self.focus_path = Path(new_path);
    }

    /// Close the focused pane. The sibling takes over the space.
    pub fn close_focused(&mut self) {
        if self.focus_path.0.is_empty() {
            return; // can't close the last pane
        }
        let mut parent = self.focus_path.0.clone();
        let closed_side = parent.pop().unwrap();
        close_at(&mut self.root, &parent, closed_side);
        self.focus_path = Path(parent);
    }

    pub fn set_focused_tab(&mut self, tab: Tab) {
        let path = self.focus_path.0.clone();
        set_leaf(&mut self.root, &path, tab);
    }

    fn all_paths(&self) -> Vec<Path> {
        let mut out = vec![];
        collect_paths(&self.root, &mut vec![], &mut out);
        out
    }
}

// ── tree traversal helpers ────────────────────────────────────────────────────

fn get_leaf_tab(node: &Tile, path: &[Side]) -> Option<Tab> {
    match (node, path) {
        (Tile::Leaf(t), []) => Some(*t),
        (Tile::Split { left, right, .. }, [head, rest @ ..]) => {
            let child = if *head == Side::Left { left } else { right };
            get_leaf_tab(child, rest)
        }
        _ => None,
    }
}

fn collect_tabs(node: &Tile, out: &mut Vec<Tab>) {
    match node {
        Tile::Leaf(t) => out.push(*t),
        Tile::Split { left, right, .. } => {
            collect_tabs(left, out);
            collect_tabs(right, out);
        }
    }
}

fn collect_paths(node: &Tile, current: &mut Vec<Side>, out: &mut Vec<Path>) {
    match node {
        Tile::Leaf(_) => out.push(Path(current.clone())),
        Tile::Split { left, right, .. } => {
            current.push(Side::Left);
            collect_paths(left, current, out);
            current.pop();
            current.push(Side::Right);
            collect_paths(right, current, out);
            current.pop();
        }
    }
}

fn split_at(node: &mut Tile, path: &[Side], dir: SplitDir, new_tab: Tab) {
    match path {
        [] => {
            let old = std::mem::replace(node, Tile::Leaf(Tab::Weather)); // placeholder
            *node = Tile::Split {
                dir,
                ratio: 0.5,
                left: Box::new(old),
                right: Box::new(Tile::Leaf(new_tab)),
            };
        }
        [head, rest @ ..] => {
            if let Tile::Split { left, right, .. } = node {
                let child = if *head == Side::Left { left } else { right };
                split_at(child, rest, dir, new_tab);
            }
        }
    }
}

fn close_at(node: &mut Tile, parent_path: &[Side], closed_side: Side) {
    match parent_path {
        [] => {
            // `node` is the split whose child we're removing; replace it with the sibling.
            if let Tile::Split { left, right, .. } = node {
                let sibling = match closed_side {
                    Side::Left => std::mem::replace(right.as_mut(), Tile::Leaf(Tab::Weather)),
                    Side::Right => std::mem::replace(left.as_mut(), Tile::Leaf(Tab::Weather)),
                };
                *node = sibling;
            }
        }
        [head, rest @ ..] => {
            if let Tile::Split { left, right, .. } = node {
                let child = if *head == Side::Left { left } else { right };
                close_at(child, rest, closed_side);
            }
        }
    }
}

fn set_leaf(node: &mut Tile, path: &[Side], tab: Tab) {
    match (node, path) {
        (Tile::Leaf(t), []) => *t = tab,
        (Tile::Split { left, right, .. }, [head, rest @ ..]) => {
            let child = if *head == Side::Left { left } else { right };
            set_leaf(child, rest, tab);
        }
        _ => {}
    }
}
