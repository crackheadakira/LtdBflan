use std::path::Path;

use nnbfl::{bflyt::file::Bflyt, core::VersionFormat, ui2d::types::Vector2f};
use tomolib::formats::bntx::Bntx;

use crate::{
    archive_browser::ArchiveEntry,
    edit_history::EditHistory,
    pane_tree::{DirtyFlags, PaneTree},
};

const EDIT_HISTORY_LIMIT: usize = 20;

pub struct BflytView {
    pub tree: PaneTree,
    pub is_centered: bool,
    pub parts_size: Vector2f,
    pub file_name: String,
    pub version: VersionFormat,
    pub history: EditHistory,
}

impl BflytView {
    pub fn reset_to_base(&mut self) {
        self.tree.for_each_mut(|node| {
            node.textured_quad = node.base_textured_quad.clone();
            node.dirty
                .insert(DirtyFlags::TRANSFORM | DirtyFlags::MATERIAL | DirtyFlags::VERTICES);
        });

        self.tree.recompute_dirty();
    }

    pub fn descendants(&self, pane_idx: usize) -> Vec<usize> {
        self.tree.descendants(pane_idx)
    }
}

pub fn build_view(
    file: Bflyt,
    blarc_dir: Option<&Path>,
    file_name: String,
    has_bntx: bool,
    archive_entries: Option<&[ArchiveEntry]>,
    discovered_bntxs: Vec<Bntx>,
) -> BflytView {
    let is_centered = file.layout.is_centered;
    let parts_size = Vector2f {
        x: file.layout.parts_width,
        y: file.layout.parts_height,
    };
    let version = file.version;

    let tree = PaneTree::from_bflyt(
        file,
        blarc_dir,
        file_name.clone(),
        has_bntx,
        archive_entries,
        discovered_bntxs,
    );

    BflytView {
        tree,
        is_centered,
        file_name,
        parts_size,
        version,
        history: EditHistory::new(EDIT_HISTORY_LIMIT),
    }
}
