//! Rendering-only E2E fixture for private screens. A passive developer client
//! must explicitly opt in. No credentials, queue state, deck or host is installed.
use super::*;
use crate::rank_art::Promotion;
use api::{QueueState, Rank};

pub(super) fn install(app: &mut App) {
    if std::env::var("BAYLEE_E2E_RANKED_FIXTURE").as_deref() == Ok("1")
        && std::env::var_os("BAYLEE_DEV_PASSIVE").is_some()
        && std::env::var_os("BAYLEE_DEV_CONTROL").is_some()
    {
        app.add_systems(
            Update,
            render
                .after(draw::patch)
                .run_if(in_state(crate::DuelPhase::Closed)),
        );
    }
}

fn render(
    mut commands: Commands,
    state: Res<LobbyState>,
    fonts: Option<Res<crate::hud::UiFonts>>,
    pages: Query<(Entity, &draw::Page)>,
    mut previous: Local<Option<(Entity, Format, u64)>>,
    mut promotions: MessageWriter<Promotion>,
) {
    let (Ok((page, desk)), Some(fonts)) = (pages.single(), fonts.as_deref()) else {
        return;
    };
    let format = state.ranked.format;
    // Refresh the fixture if normal public-board polling repainted the desk.
    let key = (page, format, state.ranked_net.revision);
    if *previous == Some(key) {
        return;
    }
    let changed = previous.is_none_or(|(p, f, _)| p != page || f != format);
    *previous = Some(key);
    let lang = state.lobby.lang();
    let kit = Kit {
        fonts,
        m: desk.metrics,
        german: lang == Lang::De,
    };
    let rank = |league, division| Rank {
        league,
        division,
        champion_place: None,
    };
    let player_before = rank(League::Gold, Some(2));
    let player_after = rank(League::Gold, Some(1));
    let deck_before = rank(League::Gold, Some(1));
    let deck_after = rank(League::Platinum, Some(4));
    let settled = format == Format::Commander;
    let own = if settled {
        (player_after, 10, Some(1810))
    } else {
        (rank(League::Unranked, None), 7, None)
    };
    let mut view = queue::look(&state, Some(own));
    view.eligible = true;
    view.queue = if settled {
        view.deck = Some((uuid::Uuid::nil(), "Solstice".to_owned()));
        QueueState::Finished {
            key: [0; 16],
            match_id: [4; 16],
            format,
            outcome: api::Outcome::FirstWins,
            seat: 0,
            elo: 1810,
            delta: 12,
            before: player_before,
            after: player_after,
            deck: api::DeckResult {
                before: deck_before,
                after: deck_after,
                elo: 1845,
                delta: 11,
                placement_games: 10,
            },
        }
    } else {
        QueueState::Waiting {
            key: [0; 16],
            format,
            queued_at: time::OffsetDateTime::now_utc().unix_timestamp() - 37,
        }
    };
    commands.entity(desk.queue).despawn_children();
    queue::draw(&mut commands, desk.queue, kit, &state, &view);
    if !changed || !settled {
        return;
    }
    for (before, after, track) in [
        (player_before, player_after, Track::Player),
        (deck_before, deck_after, Track::Deck),
    ] {
        if let Some(promotion) = receipt::notice(before, after, track, format, lang) {
            promotions.write(promotion);
        }
    }
}
