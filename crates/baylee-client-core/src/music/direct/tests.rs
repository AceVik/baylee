use super::*;
use crate::test_support::{ViewBuilder, token};
use baylee_core::ids::{Defender, ObjectId};
use baylee_view::AttackerView;

fn quiet() -> PlayerView {
    ViewBuilder::new(2).build()
}

fn at_table(view: &PlayerView, memory: &mut Memory) -> ScoreRequest {
    direct(
        Place::Table,
        Some(view),
        None,
        MusicTheme::Thorn,
        memory,
        0.016,
    )
}

/// The chosen theme rides in the request; "rotating" starts where the seed
/// says and moves on at every table that opens.
#[test]
fn the_theme_is_chosen_and_rotates_by_table() {
    let mut memory = Memory::default();
    for (choice, theme) in [
        (MusicTheme::Ember, Theme::Ember),
        (MusicTheme::Glass, Theme::Glass),
        (MusicTheme::Thorn, Theme::Thorn),
        (MusicTheme::Tide, Theme::Tide),
        (MusicTheme::Star, Theme::Star),
    ] {
        let request = direct(Place::Lobby, None, None, choice, &mut memory, 0.016);
        assert_eq!(request.theme, theme);
        assert_eq!(ScoreRequest::unpack(request.pack()).theme, theme);
    }
    let rotating = MusicTheme::Rotating;
    let mut memory = Memory::seeded(1);
    let first = direct(Place::Lobby, None, None, rotating, &mut memory, 0.016).theme;
    assert_eq!(first, Theme::Glass, "the seed picks the first");
    direct(Place::Opening, None, None, rotating, &mut memory, 0.016);
    let view = quiet();
    let second = direct(
        Place::Table,
        Some(&view),
        None,
        rotating,
        &mut memory,
        0.016,
    )
    .theme;
    assert_eq!(second, Theme::Thorn, "the next table, the next theme");
    let back = direct(Place::Lobby, None, None, rotating, &mut memory, 0.016).theme;
    assert_eq!(
        back, second,
        "it keeps its theme back in the lobby until the next table"
    );
}

fn attack(view: &mut PlayerView, creatures: u32) {
    view.combat.attackers = (0..creatures)
        .map(|slot| AttackerView {
            creature: ObjectId::new(100 + slot, 0),
            defending: Defender::Player(PlayerId::new(1)),
            blocked: false,
        })
        .collect();
}

/// The request survives its word: every field, and a tension that is no
/// number is none.
#[test]
fn a_request_packs_into_one_word() {
    let request = ScoreRequest {
        scene: Scene::Defeat,
        tension: 0.637,
        combat: true,
        big_spell: false,
        low_life: true,
        lethal: false,
        own_turn: true,
        about_to_lose: true,
        hunt_mine: true,
        hunts: 13,
        monarchs: 2,
        spells: 15,
        arrivals: 7,
        turn_seat: 5,
        theme: Theme::Tide,
    };
    assert_eq!(ScoreRequest::unpack(request.pack()), request);
    let nan = ScoreRequest {
        tension: f32::NAN,
        ..ScoreRequest::default()
    };
    assert!(ScoreRequest::unpack(nan.pack()).tension.abs() < f32::EPSILON);
    let loud = ScoreRequest {
        tension: 3.0,
        ..ScoreRequest::default()
    };
    assert!((ScoreRequest::unpack(loud.pack()).tension - 1.0).abs() < f32::EPSILON);
}

/// The screens: the front door, the lobby and the builder each their scene;
/// a table opening counts one arrival; a result is its ending, whatever the
/// phase says.
#[test]
fn every_place_has_its_scene() {
    let mut memory = Memory::default();
    for (place, scene) in [
        (Place::FrontDoor, Scene::FrontDoor),
        (Place::Lobby, Scene::Lobby),
        (Place::Build, Scene::Build),
        (Place::Opening, Scene::Opening),
    ] {
        assert_eq!(
            direct(place, None, None, MusicTheme::Thorn, &mut memory, 0.016).scene,
            scene
        );
    }
    assert_eq!(memory.arrivals, 1, "one table opened");
    direct(
        Place::Opening,
        None,
        None,
        MusicTheme::Thorn,
        &mut memory,
        0.016,
    );
    assert_eq!(
        memory.arrivals, 1,
        "an opening is one arrival however long it takes"
    );
    let view = quiet();
    assert_eq!(at_table(&view, &mut memory).scene, Scene::Table);
    for (ending, scene) in [
        (Ending::of(Some(true)), Scene::Victory),
        (Ending::of(None), Scene::Draw),
        (Ending::of(Some(false)), Scene::Defeat),
    ] {
        let request = direct(
            Place::Finished,
            Some(&view),
            Some(ending),
            MusicTheme::Thorn,
            &mut memory,
            0.016,
        );
        assert_eq!(request.scene, scene);
    }
}

/// A quiet board is calm, and its tension is nearly nothing; it is this
/// seat's turn, or another's, by the view's active seat.
#[test]
fn a_quiet_board_is_calm() {
    let mut view = quiet();
    let request = at_table(&view, &mut Memory::default());
    assert!(request.tension < 0.1, "{}", request.tension);
    assert!(request.own_turn && !request.combat && !request.lethal);
    view.active = PlayerId::new(1);
    let request = at_table(&view, &mut Memory::default());
    assert!(!request.own_turn);
    assert_eq!(request.turn_seat, 1);
}

/// Each signal of design §3.1 adds what it says: combat a quarter (and a
/// twentieth for each attacker beyond two), a big spell a fifth, low life
/// three tenths (four for this seat's own), a player about to lose a fifth;
/// a lethal board is a flag, not a number.
#[test]
fn every_signal_adds_its_tension() {
    let base = at_table(&quiet(), &mut Memory::default()).tension;
    let tension = |view: &PlayerView| at_table(view, &mut Memory::default()).tension - base;
    let near = |a: f32, b: f32| (a - b).abs() < 1e-4;

    let mut view = quiet();
    attack(&mut view, 1);
    assert!(near(tension(&view), 0.25));
    attack(&mut view, 4);
    assert!(near(tension(&view), 0.35), "two attackers beyond two");

    let mut big = token(5, 1, "Fireball", 0, 0);
    big.mana_value = 6;
    let view = ViewBuilder::new(2).with_stack(vec![big]).build();
    let request = at_table(&view, &mut Memory::default());
    assert!(request.big_spell && near(tension(&view), 0.2));

    let mut view = quiet();
    view.seats[1].life = 4;
    assert!(near(tension(&view), 0.3), "an opponent's low life");
    view.seats[0].life = 5;
    assert!(near(tension(&view), 0.4), "this seat's own");
    let mut view = quiet();
    view.seats[1].poison = 8;
    assert!(at_table(&view, &mut Memory::default()).low_life);

    let mut view = quiet();
    view.seats[1].library_count = 2;
    let request = at_table(&view, &mut Memory::default());
    assert!(request.about_to_lose && near(tension(&view), 0.2));

    let mut view = ViewBuilder::new(2)
        .with_battlefield(0, vec![token(9, 0, "Giant", 12, 12)])
        .build();
    view.seats[1].life = 12;
    let request = at_table(&view, &mut Memory::default());
    assert!(request.lethal, "twelve power against twelve life");
    view.battlefield[0].summoning_sick = true;
    assert!(
        !at_table(&view, &mut Memory::default()).lethal,
        "a creature just arrived attacks no one"
    );
    view.battlefield[0].summoning_sick = false;
    view.battlefield[0].status = baylee_view::ObjectStatus::TAPPED;
    assert!(
        !at_table(&view, &mut Memory::default()).lethal,
        "nor does a tapped one"
    );
}

/// The horn is asked for on the edge of an attack, once a turn at most,
/// and remembers whose attack it was; the monarch changing hands and a big
/// spell arriving are accents counted once each.
#[test]
fn accents_are_counted_once() {
    let mut memory = Memory::default();
    let mut view = quiet();
    at_table(&view, &mut memory);
    view.seq += 1;
    attack(&mut view, 2);
    let request = at_table(&view, &mut memory);
    assert_eq!(request.hunts, 1);
    assert!(request.hunt_mine);
    // The same attack in later views is no new hunt.
    view.seq += 1;
    assert_eq!(at_table(&view, &mut memory).hunts, 1);
    // Combat ends and a second attack comes in the same turn: no new call.
    view.seq += 1;
    view.combat.attackers.clear();
    at_table(&view, &mut memory);
    view.seq += 1;
    attack(&mut view, 1);
    assert_eq!(at_table(&view, &mut memory).hunts, 1, "one call a turn");
    // The next turn, the opponent's attack.
    view.seq += 1;
    view.combat.attackers.clear();
    view.turn += 1;
    view.active = PlayerId::new(1);
    at_table(&view, &mut memory);
    view.seq += 1;
    attack(&mut view, 1);
    let request = at_table(&view, &mut memory);
    assert_eq!(request.hunts, 2);
    assert!(!request.hunt_mine);

    view.seq += 1;
    view.monarch = Some(PlayerId::new(1));
    assert_eq!(at_table(&view, &mut memory).monarchs, 1);
    view.seq += 1;
    assert_eq!(at_table(&view, &mut memory).monarchs, 1);

    let mut big = token(5, 1, "Fireball", 0, 0);
    big.mana_value = 7;
    view.seq += 1;
    view.stack = vec![big];
    assert_eq!(at_table(&view, &mut memory).spells, 1);
    view.seq += 1;
    assert_eq!(
        at_table(&view, &mut memory).spells,
        1,
        "the same spell is one accent"
    );
}

/// Activity is a burst on a new snapshot only, decaying over seconds; a
/// repeated snapshot is not counted twice; leaving the table forgets it.
#[test]
fn activity_decays_and_a_repeated_snapshot_is_not_counted() {
    let mut memory = Memory::default();
    let mut view = quiet();
    let calm = direct(
        Place::Table,
        Some(&view),
        None,
        MusicTheme::Thorn,
        &mut memory,
        0.1,
    )
    .tension;
    view.seq += 1;
    view.seats[0].life -= 7;
    let peak = direct(
        Place::Table,
        Some(&view),
        None,
        MusicTheme::Thorn,
        &mut memory,
        0.1,
    )
    .tension;
    let again = direct(
        Place::Table,
        Some(&view),
        None,
        MusicTheme::Thorn,
        &mut memory,
        0.0,
    )
    .tension;
    let later = direct(
        Place::Table,
        Some(&view),
        None,
        MusicTheme::Thorn,
        &mut memory,
        9.0,
    )
    .tension;
    assert!(peak > calm + 0.2, "{calm} → {peak}");
    assert!(
        (again - peak).abs() < 1e-6,
        "the same snapshot again adds nothing"
    );
    assert!(later < peak && later > calm, "decays: {later}");
    direct(
        Place::Lobby,
        None,
        None,
        MusicTheme::Thorn,
        &mut memory,
        0.1,
    );
    assert!(memory.activity.abs() < f32::EPSILON && memory.seq.is_none());
}
