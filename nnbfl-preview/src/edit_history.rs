use crate::pane_tree::{PaneNode, PaneTree};

pub enum PaneEdit {
    Delete {
        target_idx: usize,
    },
    Duplicate {
        source_idx: usize,
    },
    #[allow(dead_code)]
    Insert {
        parent_idx: Option<usize>,
        position: usize,
        node: PaneNode,
    },
}

impl PaneEdit {
    pub fn apply(self, tree: &mut PaneTree) -> Option<AppliedCommand> {
        match self {
            PaneEdit::Delete { target_idx } => {
                let (parent_idx, position) = tree.sibling_position(target_idx)?;
                let node = tree.remove_node(target_idx)?;
                Some(AppliedCommand::Removed(PendingRemoval {
                    parent_idx,
                    position,
                    node,
                }))
            }

            PaneEdit::Duplicate { source_idx } => {
                let new_idx = tree.duplicate_node(source_idx)?;
                Some(AppliedCommand::Inserted(new_idx))
            }

            PaneEdit::Insert {
                parent_idx,
                position,
                node,
            } => {
                let idx = tree.insert_node_at(parent_idx, position, node);
                Some(AppliedCommand::Inserted(idx))
            }
        }
    }
}

pub struct PendingRemoval {
    parent_idx: Option<usize>,
    position: usize,
    node: PaneNode,
}

pub enum AppliedCommand {
    Inserted(usize),
    Removed(PendingRemoval),
}

impl AppliedCommand {
    pub fn invert(self, tree: &mut PaneTree) -> Option<AppliedCommand> {
        match self {
            AppliedCommand::Inserted(pane_idx) => {
                let (parent_idx, position) = tree.sibling_position(pane_idx)?;
                let node = tree.remove_node(pane_idx)?;
                Some(AppliedCommand::Removed(PendingRemoval {
                    parent_idx,
                    position,
                    node,
                }))
            }

            AppliedCommand::Removed(removal) => {
                let pane_idx =
                    tree.insert_node_at(removal.parent_idx, removal.position, removal.node);
                Some(AppliedCommand::Inserted(pane_idx))
            }
        }
    }

    pub fn resulting_pane_idx(&self) -> Option<usize> {
        match self {
            AppliedCommand::Inserted(idx) => Some(*idx),
            AppliedCommand::Removed(_) => None,
        }
    }
}

pub struct EditHistory {
    undo_stack: Vec<AppliedCommand>,
    redo_stack: Vec<AppliedCommand>,
    limit: usize,
}

impl EditHistory {
    pub fn new(limit: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            limit,
        }
    }

    pub fn perform(&mut self, tree: &mut PaneTree, edit: PaneEdit) -> Option<usize> {
        let applied = edit.apply(tree)?;
        let resulting_idx = applied.resulting_pane_idx();

        self.redo_stack.clear();
        self.push_undo(applied);

        resulting_idx
    }

    pub fn undo(&mut self, tree: &mut PaneTree) -> Option<usize> {
        let applied = self.undo_stack.pop()?;
        let inverse = applied.invert(tree)?;

        let resulting_idx = inverse.resulting_pane_idx();
        self.redo_stack.push(inverse);

        resulting_idx
    }

    pub fn redo(&mut self, tree: &mut PaneTree) -> Option<usize> {
        let applied = self.redo_stack.pop()?;
        let inverse = applied.invert(tree)?;

        let resulting_idx = inverse.resulting_pane_idx();
        self.push_undo(inverse);

        resulting_idx
    }

    fn push_undo(&mut self, applied: AppliedCommand) {
        if self.undo_stack.len() >= self.limit {
            self.undo_stack.remove(0);
        }

        self.undo_stack.push(applied);
    }
}
