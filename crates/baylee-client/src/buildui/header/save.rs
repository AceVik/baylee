//! The builder header's save control and status.

#[allow(clippy::wildcard_imports)] // the header module's shared vocabulary
use super::*;

/// Save: the primary, with the unsaved dot and its key.
pub(super) fn save_button(commands: &mut Commands, env: &Env) -> Entity {
    let kit = env.kit;
    let m = kit.m;
    let deck = env.deck();
    let live = deck.dirty() && deck.saveable() && !env.state.lobby.busy();
    let mut trailing = Vec::new();
    if deck.dirty() {
        trailing.push(glyph(
            commands,
            kit,
            mark::DOT,
            m.small * 0.55,
            tokens::GOLD,
        ));
    }
    if live
        && let Some(keys) = env.save_keys
        && let Some(cap) = controls::key_cap(commands, kit, keys)
    {
        trailing.push(cap);
    }
    let reason = if env.state.lobby.busy() {
        Phrase::SavingDeck.text(env.lang()).to_string()
    } else {
        deck.problems(env.lang())
            .into_iter()
            .find(|p| p.blocking)
            .map_or_else(|| sentence(env).2, |p| p.message)
    };
    controls::button_compact_with(
        commands,
        kit,
        Some('\u{f0c7}'),
        Phrase::BuildSave.text(env.lang()),
        Weight::Primary,
        if live { Live::Yes } else { Live::No(&reason) },
        &trailing,
        (
            Press::Build(BuildPress::SaveDeck),
            stop("save"),
            crate::tour::TourAnchor(baylee_client_core::tour::Anchor::BuildSave),
        ),
    )
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
