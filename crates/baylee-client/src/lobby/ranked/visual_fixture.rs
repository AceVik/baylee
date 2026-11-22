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
    let neutral = rank(League::Unranked, None);
    let mut view = queue::look(&state, Some((neutral, 7, None)));
    view.eligible = true;
    view.queue = QueueState::Waiting {
        key: [0; 16],
        format,
        queued_at: time::OffsetDateTime::now_utc().unix_timestamp() - 37,
    };
    commands.entity(desk.queue).despawn_children();
    queue::draw(&mut commands, desk.queue, kit, &state, &view);
    if !changed || format != Format::Commander {
        return;
    }
    for (a, step, b, next, track) in [
        (League::Gold, 2, League::Gold, 1, Track::Player),
        (League::Gold, 1, League::Platinum, 4, Track::Deck),
    ] {
        if let Some(promotion) = receipt::notice(
            rank(a, Some(step)),
            rank(b, Some(next)),
            track,
            format,
            lang,
        ) {
            promotions.write(promotion);
        }
    }
}
