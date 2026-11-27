use super::*;
use baylee_account::ranked::{Format, League, Rank, Record, Response, Season, Standing, Track};
fn season() -> Season {
    Season {
        id: [1; 16],
        name: "S1".into(),
        starts: 0,
        ends: None,
    }
}
fn rank(league: League, division: Option<u8>) -> Rank {
    Rank {
        league,
        division,
        champion_place: None,
    }
}
fn standing(track: Track, rank: Rank, elo: i32) -> Standing {
    Standing {
        format: Format::Standard,
        track,
        deck: (track == Track::Deck).then_some([2; 32]),
        elo,
        games: 0,
        wins: 0,
        losses: 0,
        draws: 0,
        placement_games: 10,
        career: Record {
            games: 12,
            wins: 9,
            losses: 2,
            draws: 1,
        },
        rank,
    }
}
fn profile() -> Profile {
    Profile {
        account: [1; 16],
        season: season(),
        formats: vec![],
        awards: vec![],
    }
}

#[test]
fn full_public_handle_must_match_both_name_and_tag() {
    let public = Response::Profile {
        season: season(),
        account: [1; 16],
        name: Some("Sam".into()),
        tag: Some(0x12),
        formats: vec![],
        awards: vec![],
        ai: None,
    };
    assert!(model::valid_handle("Sam#0012"));
    assert!(!model::valid_handle("Sam"));
    assert!(!model::valid_handle("Sam#../x"));
    assert!(!model::valid_handle("Sam#0000"));
    assert!(!model::valid_handle("Sam#00007"));
    assert!(!model::valid_handle("S#m#0012"));
    assert!(Profile::accept("Sam#0012", public.clone()).is_some());
    assert!(Profile::accept("Other#0012", public.clone()).is_none());
    assert!(Profile::accept("Sam#0013", public).is_none());
}

#[test]
fn published_account_id_resolves_an_unnamed_row_only_for_that_account() {
    let id = uuid::Uuid::now_v7().into_bytes();
    let key = OpenProfile::account(id).0;
    let public = Response::Profile {
        season: season(),
        account: id,
        name: None,
        tag: None,
        formats: vec![],
        awards: vec![],
        ai: None,
    };
    assert_eq!(
        model::target(&key),
        Some(baylee_account::ranked::ProfileTarget::Account(id))
    );
    assert!(Profile::accept(&key, public.clone()).is_some());
    assert!(Profile::accept(&model::account_key([0; 16]), public).is_none());
}

#[test]
fn an_owner_without_a_public_name_opens_by_the_authenticated_id() {
    let id = uuid::Uuid::now_v7().into_bytes();
    let mut state = LobbyState::new();
    central::enable(&mut state);
    let (mut owner, _jobs) = central::Handle::queued_for_test();
    owner.public_id = Some(id);
    state.central.as_mut().unwrap().handle = Some(owner);
    let (mut book, mut page) = (Book::default(), Page::default());
    session(&mut book, &state, &mut page);
    page.open_own(&state);
    assert_eq!(
        page.handle.as_deref(),
        Some(model::account_key(id).as_str())
    );
    assert_eq!(page.handle, book.mine);
    assert!(
        !book.own_loaded(),
        "the UI waits for Central's placement record"
    );
}

#[test]
fn frame_uses_the_best_confirmed_player_rank_across_formats() {
    let mut p = profile();
    assert_eq!(p.best(), None);
    p.formats = vec![
        standing(Track::Deck, rank(League::Champion, None), 2600),
        standing(Track::Player, rank(League::Gold, Some(1)), 1620),
        standing(Track::Player, rank(League::Platinum, Some(3)), 1760),
        standing(Track::Player, rank(League::Platinum, Some(2)), 1820),
    ];
    assert_eq!(p.best(), Some(rank(League::Platinum, Some(2))));
    assert_eq!(
        model::next(rank(League::Gold, Some(1))),
        Some(rank(League::Platinum, Some(4)))
    );
    assert_eq!(
        model::next(rank(League::Platinum, Some(2))),
        Some(rank(League::Platinum, Some(1)))
    );
}

fn reply(book: &mut Book, result: ReadResult) {
    let (sender, receiver) = mpsc::sync_channel(1);
    sender.send(result).unwrap();
    book.reading = Some(("Sam#0012".to_owned(), Mutex::new(receiver)));
    receive(book, 2.0);
}

#[test]
fn transport_failure_keeps_confirmed_data_but_a_withdrawn_identity_loses_its_frame() {
    let mut book = Book::default();
    reply(&mut book, Ok(profile()));
    let version = book.revision;
    reply(&mut book, Ok(profile()));
    assert_eq!(version, book.revision);
    reply(&mut book, Err(baylee_account::Error::Unavailable));
    assert_eq!(book.known["Sam#0012"].profile, Some(profile()));
    assert!(book.known["Sam#0012"].failed);
    reply(&mut book, Err(baylee_account::Error::Rejected));
    assert_eq!(book.known["Sam#0012"].profile, None);
}

#[test]
fn a_stopped_worker_and_a_receipt_during_an_old_read_both_recover() {
    let (sender, rx) = mpsc::channel();
    let mut book = Book {
        reading: Some(("Sam#0012".into(), Mutex::new(rx))),
        ..Book::default()
    };
    drop(sender);
    receive(&mut book, 5.0);
    assert!(book.reading.is_none());
    assert!(book.known["Sam#0012"].failed);
    book.repeat_read = true;
    reply(&mut book, Ok(profile()));
    assert!(book.known["Sam#0012"].retry_at.abs() < f64::EPSILON);
}

#[test]
fn a_partial_medal_refresh_preserves_the_last_confirmed_group_only() {
    use crate::insignia::Motif;
    use baylee_client_core::challenges::api::{BadgeKind, Tier};
    let medal = awards::Medal {
        kind: Motif::Challenge(BadgeKind::Weekly),
        tier: Tier::Silver,
        count: 4,
        next: Some(12),
    };
    let mut confirmed = Collection {
        groups: [Some(vec![medal]), None, None],
        failed: true,
    };
    confirmed.merge(Collection {
        groups: [None, Some(vec![]), Some(vec![])],
        failed: true,
    });
    assert_eq!(confirmed.groups[0].as_ref().unwrap().len(), 1);
    assert!(confirmed.groups[1].as_ref().unwrap().is_empty());
    assert!(confirmed.failed);
    confirmed.merge(Collection {
        groups: [Some(vec![]), Some(vec![]), Some(vec![])],
        failed: false,
    });
    assert!(confirmed.groups.iter().flatten().all(Vec::is_empty));
    assert!(!confirmed.failed);
}

#[test]
fn opening_loading_receiving_and_closing_preserve_the_same_sheet_and_scroll_position() {
    use crate::shellkit::surfaces::SheetScroller;
    let mut app = App::new();
    app.init_resource::<crate::hud::UiFonts>()
        .insert_resource(LobbyState::new())
        .init_resource::<Book>()
        .init_resource::<Page>()
        .add_systems(Update, draw::draw);
    app.world_mut().resource_mut::<Page>().open("Sam#0012");
    app.update();
    let world = app.world_mut();
    let root = world
        .query_filtered::<Entity, With<draw::Root>>()
        .single(world)
        .unwrap();
    let scroller = world
        .query_filtered::<Entity, With<SheetScroller>>()
        .single(world)
        .unwrap();
    world.get_mut::<ScrollPosition>(scroller).unwrap().y = 31.0;
    reply(&mut world.resource_mut::<Book>(), Ok(profile()));
    app.update();
    app.update();
    let world = app.world_mut();
    assert!(world.get::<draw::Root>(root).is_some());
    assert!((world.get::<ScrollPosition>(scroller).unwrap().y - 31.0).abs() < f32::EPSILON);
    assert!(
        world
            .query::<&Text>()
            .iter(world)
            .any(|t| t.0.contains("0/10"))
    );
    world.resource_mut::<Page>().handle = None;
    app.update();
    assert!(app.world().get_entity(root).is_err());
}
