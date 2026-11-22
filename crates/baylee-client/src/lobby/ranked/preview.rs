//! Explicit, owner-only projection for the selected shelf deck.
use super::{
    Action, Kit, Lang, League, Live, LobbyState, ORDER, Phrase, Press, Weight, api, controls, draw,
    league_name, orders, parts, queue, rank_name, tokens,
};
use api::{DeckPreview, PreviewSource};
use bevy::prelude::*;

#[derive(PartialEq, Eq)]
pub(super) struct Look {
    result: Option<DeckPreview>,
    busy: bool,
    failed: bool,
}

pub(super) fn look(state: &LobbyState) -> Look {
    let version = queue::selected(state).map(|(id, _)| (queue::effective_format(state), id));
    let result = state
        .ranked_net
        .preview
        .as_ref()
        .filter(|(format, id, _)| Some((*format, *id)) == version)
        .map(|(_, _, result)| result)
        .copied();
    Look {
        result,
        busy: state.ranked_net.preview_busy,
        failed: state.ranked_net.preview_error.is_some(),
    }
}

pub(super) fn draw(commands: &mut Commands, parent: Entity, kit: Kit, look: &Look) {
    let lang = kit.lang();
    if let Some(result) = &look.result {
        let row = parts::row(commands, kit, true);
        commands.entity(parent).add_child(row);
        if let Some(art) = crate::rank_art::RankArt::badge(
            league_name(result.rank.league, Lang::En),
            result.rank.division,
            true,
            None,
        ) {
            let icon = commands
                .spawn((
                    Node {
                        width: kit.m.px(36.0),
                        height: kit.m.px(36.0),
                        ..default()
                    },
                    art,
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(row).add_child(icon);
        }
        let rank = rank_name(result.rank, lang);
        let score = Phrase::RankedElo.fill(lang, &[&result.elo.to_string()]);
        draw::label(
            commands,
            row,
            kit,
            &format!("{rank} · {score}"),
            kit.m.text,
            tokens::ACCENT,
        );
        if result.rank.league == League::Unranked {
            let progress =
                Phrase::RankedPlacements.fill(lang, &[&result.placement_games.to_string()]);
            draw::label(commands, row, kit, &progress, kit.m.small, tokens::MUTED);
        }
        let reason = match result.source {
            PreviewSource::New => Phrase::RankedPreviewNew.text(lang).to_string(),
            PreviewSource::Recorded => Phrase::RankedPreviewRecorded.text(lang).to_string(),
            PreviewSource::Inherited => Phrase::RankedPreviewInherited.fill(
                lang,
                &[
                    &result.changed_copies.to_string(),
                    &result.compared_copies.to_string(),
                    &result.edit_loss.to_string(),
                ],
            ),
        };
        draw::label(commands, parent, kit, &reason, kit.m.small, tokens::MUTED);
    } else {
        draw::label(
            commands,
            parent,
            kit,
            Phrase::RankedPreviewDisclosure.text(lang),
            kit.m.small,
            tokens::MUTED,
        );
    }
    if look.failed {
        draw::label(
            commands,
            parent,
            kit,
            Phrase::RankedPreviewFailed.text(lang),
            kit.m.small,
            tokens::DANGER,
        );
    }
    let live = if look.busy {
        Live::No(Phrase::ChallengeLoading.text(lang))
    } else {
        Live::Yes
    };
    let title = if look.busy {
        Phrase::ChallengeLoading
    } else if look.result.is_some() {
        Phrase::RankedPreviewRefresh
    } else {
        Phrase::RankedPreview
    };
    let button = controls::button_compact(
        commands,
        kit,
        None,
        title.text(lang),
        Weight::Ghost,
        live,
        Press::Ranked(Action::Preview),
    );
    if !look.busy {
        orders::stop(commands, button, &ORDER, "preview");
    }
    commands.entity(parent).add_child(button);
}
