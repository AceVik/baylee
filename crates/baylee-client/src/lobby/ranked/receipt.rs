//! Match-local notification of a confirmed settlement.
use super::*;
use api::QueueState;

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
    let art = |r: api::Rank, is_deck: bool| {
        crate::rank_art::RankArt::badge(
            league_name(r.league, Lang::En),
            r.division,
            is_deck,
            r.champion_place.map(u32::from),
        )
    };
    for (before, after, is_deck, track) in [
        (*before, *after, false, Phrase::RankedPlayers),
        (deck.before, deck.after, true, Phrase::RankedDecks),
    ] {
        if before == after {
            continue;
        }
        let (Some(old), Some(new)) = (art(before, is_deck), art(after, is_deck)) else {
            continue;
        };
        let lang = state.lobby.lang();
        events.write(crate::rank_art::Promotion {
            before: old,
            after: new,
            label: format!(
                "{} · {} · {}",
                format_name(*format, lang),
                track.text(lang),
                rank_name(after, lang)
            ),
        });
    }
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
