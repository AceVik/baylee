//! The builder header's save control and status.

#[allow(clippy::wildcard_imports)] // the header module's shared vocabulary
use super::*;

use crate::shellkit::controls::Disabled;

/// Save: the primary, with the unsaved dot and its key.
pub(super) fn save_button(commands: &mut Commands, env: &Env) -> Entity {
    let kit = env.kit;
    let m = kit.m;
    let deck = env.deck();
    let live = deck.dirty() && deck.saveable() && !env.state.lobby.busy();
    let face = commands
        .spawn((
            Role::Button,
            Node {
                min_height: px_fixed(m.control),
                padding: UiRect::axes(m.px(if kit.german { 10.0 } else { 12.0 }), px_fixed(0.0)),
                column_gap: m.px(8.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px_fixed(1.0)),
                border_radius: BorderRadius::all(px_fixed(tokens::RADIUS_CONTROL)),
                ..default()
            },
            BackgroundColor(if live {
                tokens::PRIMARY
            } else {
                tokens::CONTROL.with_alpha(0.6)
            }),
            BorderColor::all(if live {
                tokens::PRIMARY_EDGE
            } else {
                tokens::BORDER
            }),
        ))
        .id();
    let label = commands
        .spawn((
            Text::new(Phrase::BuildSave.text(env.lang())),
            tf_bold(kit.fonts, m.text),
            TextColor(if live { tokens::INK } else { tokens::DISABLED }),
            TextLayout::no_wrap(),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(face).add_child(label);
    if deck.dirty() {
        let dot = glyph(commands, kit, mark::DOT, m.small * 0.55, tokens::GOLD);
        commands.entity(face).add_child(dot);
    }
    if live
        && let Some(keys) = env.save_keys
        && let Some(cap) = controls::key_cap(commands, kit, keys)
    {
        commands.entity(face).add_child(cap);
    }
    if live {
        commands
            .entity(face)
            .insert(crate::ambience::Feel::new(tokens::PRIMARY));
        controls::hit(
            commands,
            kit,
            face,
            (
                Press::Build(BuildPress::SaveDeck),
                stop("save"),
                crate::tour::TourAnchor(baylee_client_core::tour::Anchor::BuildSave),
            ),
        )
    } else {
        controls::hit(
            commands,
            kit,
            face,
            (
                Press::Build(BuildPress::SaveDeck),
                stop("save"),
                Disabled,
                crate::tour::TourAnchor(baylee_client_core::tour::Anchor::BuildSave),
            ),
        )
    }
}

/// What the save state says: its mark, its ink, its words, and whether it
/// offers Retry.
fn sentence(env: &Env) -> (Option<char>, Color, String, bool) {
    let deck = env.deck();
    let lang = env.lang();
    match env.ui().save {
        SaveState::Saving => (
            None,
            tokens::MUTED,
            Phrase::BuildSaving.text(lang).into(),
            false,
        ),
        SaveState::Failed if deck.dirty() => (
            Some(mark::WARN),
            tokens::DANGER,
            Phrase::BuildSaveFailed.text(lang).into(),
            true,
        ),
        _ if deck.dirty() => {
            let n = deck.changes().max(1);
            let words = if n == 1 {
                Phrase::BuildUnsavedOne.text(lang).to_string()
            } else {
                Phrase::BuildUnsavedMany.fill(lang, &[&n.to_string()])
            };
            (Some(mark::DOT), tokens::GOLD, words, false)
        }
        SaveState::Saved { minutes, .. } => (
            Some(mark::CHECK),
            tokens::INK,
            if minutes == 0 {
                Phrase::BuildSavedNow.text(lang).to_string()
            } else {
                Phrase::BuildSavedAgo.fill(lang, &[&minutes.to_string()])
            },
            false,
        ),
        _ if deck.editing().is_some() => (
            Some(mark::CHECK),
            tokens::MUTED,
            Phrase::BuildSaved.text(lang).into(),
            false,
        ),
        _ => (
            None,
            tokens::MUTED,
            Phrase::BuildNotSaved.text(lang).into(),
            false,
        ),
    }
}

/// The one save state (§7: "Unsaved · n changes / Saved ✓ · time /
/// Couldn't save · Retry"); on a narrow header its mark alone.
pub(super) fn save_state(commands: &mut Commands, env: &Env) -> Entity {
    let kit = env.kit;
    let m = kit.m;
    let (sign_mark, ink, said, retry) = sentence(env);
    let node = commands
        .spawn((
            SaveStateNode,
            // Its mark never gives way; its sentence may (the cell clips).
            Node {
                align_items: AlignItems::Center,
                column_gap: m.px(6.0),
                min_width: px_fixed(m.small * 1.2),
                flex_shrink: 1.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    if let Some(sign_mark) = sign_mark {
        let size = if sign_mark == mark::DOT {
            m.small * 0.55
        } else {
            m.small
        };
        let sign = glyph(commands, kit, sign_mark, size, ink);
        commands.entity(node).add_child(sign);
    }
    let room = matches!(m.frame, Frame::Wide | Frame::Vast | Frame::Narrow);
    if room {
        let text = cell(commands, kit, &said, m.small, ink, false);
        commands.entity(node).add_child(text);
    }
    if retry {
        let again = link(
            commands,
            kit,
            Phrase::ShellRetry.text(env.lang()),
            (Press::Build(BuildPress::SaveDeck), stop("retry")),
        );
        commands.entity(node).add_child(again);
    }
    node
}
