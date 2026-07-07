use std::path::PathBuf;

use crate::{
    archive_browser::ArchiveScan,
    ui::{DrawUi, general::UiAction},
};

#[derive(Default)]
pub struct ArchiveBrowser {
    pub is_visible: bool,
    pub layout_dir: Option<PathBuf>,

    pub archive_scan: Option<ArchiveScan>,
}

impl DrawUi<Option<UiAction>> for ArchiveBrowser {
    fn draw(&mut self, ui: &mut egui::Ui) -> Option<UiAction> {
        if !self.is_visible {
            return None;
        }

        let mut out_action = None;

        egui::Window::new("Browse Archives")
            .open(&mut self.is_visible)
            .default_width(420.0)
            .default_height(420.0)
            .collapsible(false)
            .show(ui, |ui| {
                let Some(dir) = &self.layout_dir else {
                    ui.label("Set a layout folder first (File > Set Layout Folder...).");
                    return;
                };

                ui.label(format!("Directory: {}", dir.display()));
                ui.separator();

                match &self.archive_scan {
                    None => {
                        ui.label(
                            "Not scanned yet. Scanning reads and unpacks every archive in this \
                         directory to check for BFLYT layouts, which can take a while on a \
                         large directory.",
                        );
                        if ui.button("Scan directory").clicked() {
                            out_action = Some(UiAction::StartArchiveScan);
                        }
                    }

                    Some(scan) if scan.root() != dir => {
                        ui.label("The layout folder changed since the last scan.");
                        if ui.button("Scan directory").clicked() {
                            out_action = Some(UiAction::StartArchiveScan);
                        }
                    }

                    Some(scan) => {
                        ui.horizontal(|ui| {
                            if scan.done {
                                ui.label(format!(
                                    "Found {} BFLYT-containing archive(s) out of {} scanned.",
                                    scan.entries.len(),
                                    scan.scanned
                                ));
                            } else if scan.cancelled {
                                ui.label(format!(
                                    "Cancelled after scanning {} of {}.",
                                    scan.scanned, scan.total
                                ));
                            } else {
                                ui.spinner();
                                ui.label(format!(
                                    "Scanning... {} / {}",
                                    scan.scanned,
                                    scan.total.max(scan.scanned)
                                ));
                                if ui.button("Cancel").clicked() {
                                    out_action = Some(UiAction::CancelArchiveScan);
                                }
                            }

                            if (scan.done || scan.cancelled) && ui.button("Rescan").clicked() {
                                out_action = Some(UiAction::StartArchiveScan);
                            }
                        });

                        if !scan.done && !scan.cancelled && scan.total > 0 {
                            ui.add(egui::ProgressBar::new(
                                scan.scanned as f32 / scan.total.max(1) as f32,
                            ));
                        }

                        ui.separator();

                        egui::ScrollArea::vertical().auto_shrink(false).show_rows(
                            ui,
                            24.0,
                            scan.entries.len(),
                            |ui, row_range| {
                                if scan.entries.is_empty() && scan.done {
                                    ui.weak("No BFLYT-containing archives found.");
                                }

                                for i in row_range {
                                    let entry = &scan.entries[i];

                                    ui.horizontal(|ui| {
                                        ui.label(&entry.display_name);
                                        if ui.button("Load").clicked() {
                                            out_action =
                                                Some(UiAction::LoadArchiveEntry(entry.clone()));
                                        }
                                    });
                                }
                            },
                        );
                    }
                }
            });

        out_action
    }
}
