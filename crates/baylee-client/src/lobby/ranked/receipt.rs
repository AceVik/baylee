//! Match-local notification of a confirmed settlement.
use super::{Format, Lang, LobbyState, Phrase, Track, api, format_name, league_name, rank_name};
use api::QueueState;
use bevy::prelude::*;

/// Authoritative receipts are polled repeatedly. Only a game entered by this
/// client can produce an animation, once, even after revisiting the lobby.
pub(super) fn promotion(
    state: Res<LobbyState>,
    mut seen: Local<Option<api::Id>>,
    mut events: MessageWriter<crate::rank_art::Promotion>,
    mut refresh: MessageWriter<crate::lobby::ranked_profile::RefreshOwn>,
) {
    let QueueState::Finished {
        deck,
        match_id,
        before,
        after,
        format,
        ..
    } = &state.ranked_net.queue
    else {
        return;
    };
    if state.ranked_net.played != Some(*match_id) || *seen == Some(*match_id) {
        return;
    }
    *seen = Some(*match_id);
    refresh.write(crate::lobby::ranked_profile::RefreshOwn);
    for (before, after, track) in [
        (*before, *after, Track::Player),
        (deck.before, deck.after, Track::Deck),
    ] {
        if let Some(promotion) = notice(before, after, track, *format, state.lobby.lang()) {
            events.write(promotion);
        }
    }
}

pub(super) fn notice(
    before: api::Rank,
    after: api::Rank,
    track: Track,
    format: Format,
    lang: Lang,
) -> Option<crate::rank_art::Promotion> {
    if before == after {
        return None;
    }
    let art = |r: api::Rank| {
        crate::rank_art::RankArt::badge(
            league_name(r.league, Lang::En),
            r.division,
            track == Track::Deck,
            r.champion_place.map(u32::from),
        )
    };
    let kind = match track {
        Track::Player => Phrase::ProfilePlayer,
        Track::Deck => Phrase::BuildPaneDeck,
    };
    Some(crate::rank_art::Promotion {
        before: art(before)?,
        after: art(after)?,
        label: format!(
            "{} · {} · {}",
            format_name(format, lang),
            kind.text(lang),
            rank_name(after, lang)
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_the_locally_played_finished_match_emits_each_promotion_once() {
        use crate::lobby::ranked_profile::RefreshOwn;
        use crate::rank_art::Promotion;
        let id = [4; 16];
        let rank = |league| api::Rank {
            league,
            division: None,
            champion_place: None,
        };
        let unranked = rank(api::League::Unranked);
        let paper = rank(api::League::Paper);
        let mut state = LobbyState::new();
        state.ranked_net.queue = api::QueueState::Finished {
            key: [2; 16],
            match_id: id,
            format: api::Format::Standard,
            outcome: api::Outcome::FirstWins,
            seat: 0,
            elo: 1410,
            delta: 10,
            before: unranked,
            after: paper,
            deck: api::DeckResult {
                before: unranked,
                after: paper,
                elo: 1395,
                delta: 8,
                placement_games: 10,
            },
        };
        let mut app = App::new();
        app.insert_resource(state)
            .add_message::<Promotion>()
            .add_message::<RefreshOwn>()
            .add_systems(Update, promotion);
        app.update();
        assert_eq!(
            app.world_mut()
                .resource_mut::<Messages<Promotion>>()
                .drain()
                .count(),
            0
        );
        app.world_mut()
            .resource_mut::<LobbyState>()
            .ranked_net
            .played = Some(id);
        app.update();
        assert_eq!(
            app.world_mut()
                .resource_mut::<Messages<Promotion>>()
                .drain()
                .count(),
            2
        );
        app.update();
        assert_eq!(
            app.world_mut()
                .resource_mut::<Messages<Promotion>>()
                .drain()
                .count(),
            0
        );
    }
}
