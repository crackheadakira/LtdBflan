use egui::{Context, Key, Modifiers};

use crate::ui::{
    general::{UiAction, UiState},
    timeline::TimelineState,
};

#[derive(Clone, Copy)]
pub struct KeyBind {
    pub key: Key,
    pub modifiers: Modifiers,
    pub action: Action,
    pub description: &'static str,
}

#[derive(Clone, Copy)]
pub enum Action {
    SaveAs,
    TogglePlayback,
    Deselect,
    FitTimeline,
    DeleteSelected,
    DuplicateSelected,
    Undo,
    Redo,
    Open,
    SetLayoutDirectory,
    ToggleArchiveBrowser,
}

pub const BINDINGS: &[KeyBind] = &[
    KeyBind {
        key: Key::S,
        modifiers: Modifiers::COMMAND,
        action: Action::SaveAs,
        description: "Save As...",
    },
    KeyBind {
        key: Key::Space,
        modifiers: Modifiers::NONE,
        action: Action::TogglePlayback,
        description: "Play/pause the active animation",
    },
    KeyBind {
        key: Key::Escape,
        modifiers: Modifiers::NONE,
        action: Action::Deselect,
        description: "Deselect the current pane / close menus",
    },
    KeyBind {
        key: Key::F,
        modifiers: Modifiers::NONE,
        action: Action::FitTimeline,
        description: "Reset the timeline's zoom/pan",
    },
    KeyBind {
        key: Key::Delete,
        modifiers: Modifiers::NONE,
        action: Action::DeleteSelected,
        description: "Delete the selected pane",
    },
    KeyBind {
        key: Key::Backspace,
        modifiers: Modifiers::NONE,
        action: Action::DeleteSelected,
        description: "Delete the selected pane",
    },
    KeyBind {
        key: Key::D,
        modifiers: Modifiers::COMMAND,
        action: Action::DuplicateSelected,
        description: "Duplicate the selected pane",
    },
    KeyBind {
        key: Key::Z,
        modifiers: Modifiers::COMMAND,
        action: Action::Undo,
        description: "Undo",
    },
    KeyBind {
        key: Key::Z,
        modifiers: Modifiers {
            command: true,
            shift: true,
            ..Modifiers::NONE
        },
        action: Action::Redo,
        description: "Redo",
    },
    KeyBind {
        key: Key::O,
        modifiers: Modifiers::COMMAND,
        action: Action::Open,
        description: "Open File",
    },
    KeyBind {
        key: Key::O,
        modifiers: Modifiers {
            command: true,
            shift: true,
            ..Modifiers::NONE
        },
        action: Action::SetLayoutDirectory,
        description: "Set Layout Directory",
    },
    KeyBind {
        key: Key::B,
        modifiers: Modifiers::COMMAND,
        action: Action::ToggleArchiveBrowser,
        description: "Open/Close Archive Browser",
    },
];

pub fn handle(ctx: &Context, state: &mut UiState, timeline: Option<&mut TimelineState>) {
    let mut action_to_apply = None;

    ctx.input_mut(|i| {
        for bind in BINDINGS {
            if i.modifiers.matches_exact(bind.modifiers) && i.key_pressed(bind.key) {
                action_to_apply = Some(*bind);
                break;
            }
        }
    });

    if let Some(bind) = action_to_apply {
        if ctx.egui_wants_keyboard_input() {
            return;
        };

        let fired = ctx.input_mut(|i| i.consume_key(bind.modifiers, bind.key));

        if fired {
            if bind.modifiers.is_none() && ctx.egui_wants_keyboard_input() {
                return;
            }

            apply(bind.action, state, timeline);
        }
    }
}

fn apply(action: Action, state: &mut UiState, timeline: Option<&mut TimelineState>) {
    match action {
        Action::TogglePlayback => {
            if let Some(timeline) = timeline
                && let Some(idx) = timeline.anim_player.active
                && let Some(anim) = timeline.anim_player.anims.get_mut(idx)
            {
                anim.playing = !anim.playing;
            }
        }

        Action::Deselect => {
            state.pane_tree_view.selected_pane = None;
            state.context_menu.is_open = false;
        }

        Action::FitTimeline => {
            if let Some(timeline) = timeline {
                timeline.zoom = 1.0;
                timeline.pan_frame = 0.0;
            }
        }

        Action::DeleteSelected => {
            if let Some(pane_idx) = state.pane_tree_view.selected_pane {
                state.pending_action = Some(UiAction::DeletePane(pane_idx));
            }
        }

        Action::DuplicateSelected => {
            if let Some(pane_idx) = state.pane_tree_view.selected_pane {
                state.pending_action = Some(UiAction::DuplicatePane(pane_idx));
            }
        }

        Action::Undo => state.pending_action = Some(UiAction::Undo),
        Action::Redo => state.pending_action = Some(UiAction::Redo),
        Action::Open => state.pending_action = Some(UiAction::LoadFile),
        Action::SaveAs => state.pending_action = Some(UiAction::SaveFile),
        Action::SetLayoutDirectory => state.pending_action = Some(UiAction::SetBlarcDir),
        Action::ToggleArchiveBrowser => {
            state.archive_browser.is_visible = !state.archive_browser.is_visible
        }
    }
}
