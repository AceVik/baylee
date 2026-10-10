//! Spectators at the table (`docs/protocol.md` §"Spectators"): a line at
//! the top saying how many watch, and to a spectator that it is watching;
//! and the spectator's way out.

use super::{DetachedHud, UiFonts, tf};
use crate::{Duel, DuelCommand};
use baylee_client_core::Lang;
use baylee_client_core::i18n::Phrase;
use bevy::prelude::*;

/// The line at the top of the table.
#[derive(Component)]
pub(crate) struct SpectatorLine;

/// What the line says, or `None` when there is nothing to say.
pub(crate) fn spectator_words(lang: Lang, watching: bool, count: u32) -> Option<String> {
    let counted = (count > 0).then(|| Phrase::SpectatorCount.fill(lang, &[&count.to_string()]));
    match (watching, counted) {
        (true, Some(counted)) => Some(format!(
            "{} \u{b7} {counted}",
            Phrase::YouAreWatching.text(lang)
        )),
        (true, None) => Some(Phrase::YouAreWatching.text(lang).to_string()),
        (false, counted) => counted,
    }
}

/// Keeps the line in step with the duel: spawned when there is something
/// to say, rewritten when it changes, gone when there is not.
pub(crate) fn sync_spectator_line(
    mut commands: Commands,
    duel: Res<Duel>,
    fonts: Res<UiFonts>,
    settings: Res<crate::settings::ClientSettings>,
    mut line: Query<(Entity, &mut Text), With<SpectatorLine>>,
) {
    let lang = Lang::of(&settings.lang);
    let words = spectator_words(lang, duel.watching, duel.spectators);
    match (words, line.single_mut()) {
        (Some(words), Ok((_, mut text))) => {
            if text.0 != words {
                text.0 = words;
            }
        }
        (Some(words), Err(_)) => {
            commands.spawn((
                SpectatorLine,
                DetachedHud,
                Node {
                    position_type: PositionType::Absolute,
                    top: px(6),
                    width: percent(100),
                    ..default()
                },
                Text::new(words),
                TextLayout::justify(Justify::Center),
                tf(&fonts, 14.0),
                TextColor(Color::srgba(0.92, 0.94, 0.98, 0.85)),
                GlobalZIndex(5),
                Pickable::IGNORE,
            ));
        }
        (None, Ok((entity, _))) => commands.entity(entity).despawn(),
        (None, Err(_)) => {}
    }
}

/// A spectator's *Stop watching* closes the table; nothing is sent, since a
/// spectator has nothing to say to the game.
pub(crate) fn leave_watching(mut duel: ResMut<Duel>, mut closes: MessageWriter<DuelCommand>) {
    if duel.leave_asked {
        duel.leave_asked = false;
        closes.write(DuelCommand::Close);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_line_says_who_watches_in_both_languages() {
        assert_eq!(spectator_words(Lang::En, false, 0), None);
        assert_eq!(
            spectator_words(Lang::De, false, 3).as_deref(),
            Some("3 Zuschauer")
        );
        assert_eq!(
            spectator_words(Lang::En, true, 2).as_deref(),
            Some("You are watching \u{b7} 2 watching")
        );
        assert_eq!(
            spectator_words(Lang::De, true, 0).as_deref(),
            Some("Du schaust zu")
        );
    }
}
