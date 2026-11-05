//! The builder header's save control and status.

#[allow(clippy::wildcard_imports)] // the header module's shared vocabulary
use super::*;

/// Save: icon-only header tool button with tooltip and dirty dot.
pub(super) fn save_button(commands: &mut Commands, env: &Env) -> Entity {
    let kit = env.kit;
    let m = kit.m;
    let deck = env.deck();
    let lang = env.lang();
    let live = deck.saveable() && !env.state.lobby.busy();
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
    let reason = if env.state.lobby.busy() {
        Phrase::SavingDeck.text(lang).to_string()
    } else {
        deck.problems(lang)
            .into_iter()
            .find(|p| p.blocking)
            .map_or_else(|| sentence(env).2, |p| p.message)
    };
    let save_base = match env.save_keys {
        Some(keys) if !keys.is_empty() => format!("{} ({keys})", Phrase::BuildSave.text(lang)),
        _ => Phrase::BuildSave.text(lang).to_string(),
    };
    let save_hint = if live {
        save_base
    } else {
        format!("{save_base} — {reason}")
    };
    super::tool_icon_button(
        commands,
        kit,
        '\u{f0c7}',
        live,
        save_hint,
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
