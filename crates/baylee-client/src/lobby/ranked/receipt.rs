//! Match-local notification of a confirmed settlement.
use super::{Format, Lang, LobbyState, Phrase, Track, api, format_name, league_name, rank_name};
use crate::lobby::advent::RefreshInventory;
use api::QueueState;
use baylee_client_core::aiplayers::{Counted, Verdict};
use bevy::prelude::*;

/// Authoritative receipts are polled repeatedly. Only a game entered by this
/// client can produce an animation, once, even after revisiting the lobby.
pub(super) fn promotion(
    state: Res<LobbyState>,
    mut seen: Local<Option<api::Id>>,
    mut events: MessageWriter<crate::rank_art::Promotion>,
    mut refresh: MessageWriter<crate::lobby::ranked_profile::RefreshOwn>,
    mut collection: MessageWriter<RefreshInventory>,
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
    if before != after {
        collection.write(RefreshInventory);
    }
    for (before, after, track) in [
        (*before, *after, Track::Player),
        (deck.before, deck.after, Track::Deck),
    ] {
        if let Some(promotion) = notice(before, after, track, *format, state.lobby.lang()) {
            events.write(promotion);
        }
    }
}

/// How a settled game went from this player's seat.
fn verdict(outcome: api::Outcome, seat: u8) -> Verdict {
    match (outcome, seat) {
        (api::Outcome::FirstWins, 0) | (api::Outcome::SecondWins, 1) => Verdict::Won,
        (api::Outcome::Draw, _) => Verdict::Drawn,
        _ => Verdict::Lost,
    }
}

/// A game this client entered against an AI player counts, once, into the
/// signed-in account's record on this device ("Against AIs" on the own
/// profile), and a first win against that AI is marked on the result card.
pub(super) fn against_ai(
    mut state: ResMut<LobbyState>,
    mut settings: Option<ResMut<crate::settings::ClientSettings>>,
    mut seen: Local<Option<api::Id>>,
) {
    let QueueState::Finished {
        match_id,
        outcome,
        seat,
        opponent_ai: Some(badge),
        ..
    } = &state.ranked_net.queue
    else {
        return;
    };
    if state.ranked_net.played != Some(*match_id) || *seen == Some(*match_id) {
        return;
    }
    let (game, verdict, badge) = (*match_id, verdict(*outcome, *seat), badge.clone());
    *seen = Some(game);
    let Some(account) = state
        .central_handle()
        .and_then(|handle| handle.public_id)
        .map(|id| uuid::Uuid::from_bytes(id).to_string())
    else {
        return;
    };
    let Some(settings) = settings.as_mut() else {
        return;
    };
    let counted = settings
        .ranked_against_ai
        .of_mut(&account)
        .count(game, &badge, verdict);
    if counted == Counted::Again {
        return;
    }
    settings.save();
    if counted == Counted::FirstWin {
        let ui = &mut state.bypass_change_detection().ranked;
        ui.first_win = Some(game);
        ui.revision = ui.revision.wrapping_add(1);
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
            opponent_ai: None,
        };
        let mut app = App::new();
        app.insert_resource(state)
            .add_message::<Promotion>()
            .add_message::<RefreshOwn>()
            .add_message::<RefreshInventory>()
            .add_systems(Update, promotion);
        app.update();
        assert!(
            app.world()
                .resource::<Messages<RefreshInventory>>()
                .is_empty()
        );
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
                .resource_mut::<Messages<RefreshInventory>>()
                .drain()
                .count(),
            1
        );
        assert_eq!(
            app.world_mut()
                .resource_mut::<Messages<Promotion>>()
                .drain()
                .count(),
            2
        );
        app.update();
        assert!(
            app.world()
                .resource::<Messages<RefreshInventory>>()
                .is_empty()
        );
        assert_eq!(
            app.world_mut()
                .resource_mut::<Messages<Promotion>>()
                .drain()
                .count(),
            0
        );
    }
}

#[cfg(test)]
mod against_ai_tests {
    use super::*;
    use crate::lobby::central;
    use baylee_client_core::aiplayers::{AiBadge, AiKind, Tally};

    fn finished(match_id: api::Id, outcome: api::Outcome) -> QueueState {
        let rank = api::Rank {
            league: api::League::Gold,
            division: Some(2),
            champion_place: None,
        };
        QueueState::Finished {
            key: [2; 16],
            match_id,
            format: api::Format::Standard,
            outcome,
            seat: 0,
            elo: 1810,
            delta: 12,
            before: rank,
            after: rank,
            deck: api::DeckResult {
                before: rank,
                after: rank,
                elo: 1800,
                delta: 10,
                placement_games: 10,
            },
            opponent_ai: Some(AiBadge {
                kind: AiKind::House,
                label: "Haus-KI · Expert".into(),
            }),
        }
    }

    /// A game this client entered against an AI counts once into the
    /// account's record here, and its first win is marked for the card; a
    /// game it did not enter counts nothing.
    #[test]
    fn a_game_against_an_ai_counts_once_and_marks_its_first_win() {
        let account = uuid::Uuid::now_v7();
        let mut state = LobbyState::new();
        central::enable(&mut state);
        let (mut handle, _jobs) = central::Handle::queued_for_test();
        handle.public_id = Some(*account.as_bytes());
        state.central.as_mut().unwrap().handle = Some(handle);
        let game = *uuid::Uuid::now_v7().as_bytes();
        state.ranked_net.queue = finished(game, api::Outcome::FirstWins);
        let mut app = App::new();
        app.insert_resource(state)
            .insert_resource(crate::settings::ClientSettings::default())
            .add_systems(Update, against_ai);
        app.update();
        let key = account.to_string();
        let counted = |app: &App| {
            app.world()
                .resource::<crate::settings::ClientSettings>()
                .ranked_against_ai
                .of(&key)
                .map(baylee_client_core::aiplayers::Record::by_kind)
        };
        assert_eq!(
            counted(&app),
            None,
            "a game not entered here counts nothing"
        );
        app.world_mut()
            .resource_mut::<LobbyState>()
            .ranked_net
            .played = Some(game);
        app.update();
        app.update();
        assert_eq!(
            counted(&app),
            Some(vec![(
                AiKind::House,
                Tally {
                    wins: 1,
                    losses: 0,
                    draws: 0
                }
            )]),
            "counted once however often the receipt is read"
        );
        assert_eq!(
            app.world().resource::<LobbyState>().ranked.first_win,
            Some(game)
        );
    }
}
