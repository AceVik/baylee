//! Daybound and nightbound (CR 702.145) at the table, with the five cards
//! in the pool that print them.
//!
//! The designation's own arithmetic is tested in `day_night_tests` with no
//! card at all. What is here needs a permanent that can turn over: the four
//! continuous checks (CR 702.145c–g), the "enters transformed" clause, and
//! the two shapes the transform has to refuse.

use super::testkit::*;
use super::*;
use crate::turn::DayNight;

fn plains() -> baylee_core::ids::CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}
fn mountain() -> baylee_core::ids::CardIndex {
    card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685")
}
/// Tavern Ruffian // Tavern Smasher — {3}{R}, a 2/5 by day and a 6/5 by
/// night with nothing else printed on either face.
fn tavern_ruffian() -> baylee_core::ids::CardIndex {
    card_index("73a3b9a1-37a0-469a-9557-8c118a1ee78f")
}
/// Tireless Hauler // Dire-Strain Brawler — {4}{G}, vigilance on both
/// faces, and the back face is green by its color indicator alone.
fn tireless_hauler() -> baylee_core::ids::CardIndex {
    card_index("c31e9db3-5d9d-470a-871a-b4b5b0536db5")
}

fn seat0(engine: &mut Engine<RegistryLookup>) -> &mut crate::state::GameState {
    engine
        .dev_state_mut(PlayerId::new(0))
        .expect("the harness trusts itself")
}

/// Nudges the machine so the fixpoint runs again without changing anything
/// a rule reads: passing priority is the cheapest legal way to do it.
fn settle(engine: &mut Engine<RegistryLookup>) {
    if let Pending::Priority { player, .. } = engine.pending().clone() {
        engine
            .apply(player, PlayerAction::PassPriority)
            .expect("a seat may always pass");
    }
}

fn face_of(engine: &Engine<RegistryLookup>, id: baylee_core::ids::ObjectId) -> u8 {
    engine.state().object(id).expect("still there").face_index
}

/// CR 702.145d: any time a player controls a permanent with daybound and it
/// is neither day nor night, it becomes day. This is how a game gets its
/// first designation without anything saying "it becomes day" out loud.
#[test]
fn a_daybound_permanent_makes_a_fresh_game_day() {
    let mut engine = Duel::new(910, plains())
        .battlefield(0, &[tavern_ruffian()])
        .start();
    keep_mulligans(&mut engine);
    assert_eq!(engine.state().day_night, Some(DayNight::Day));
    let wolf = on_battlefield(&engine, PlayerId::new(0), tavern_ruffian()).expect("in play");
    assert_eq!(face_of(&engine, wolf), 0, "day leaves the front face up");
    assert_eq!(pt(&engine, wolf), (2, 5));
}

/// CR 702.145g: a nightbound permanent with no daybound permanent anywhere
/// on the battlefield makes a designationless game night. The condition is
/// the whole rule — the same permanent's own card prints daybound on its
/// other face, and if keywords were read per *card* rather than per face
/// this could never fire.
#[test]
fn a_lone_nightbound_permanent_makes_a_fresh_game_night() {
    let mut engine = Duel::new(911, plains())
        .battlefield(0, &[tavern_ruffian()])
        .start();
    keep_mulligans(&mut engine);
    let wolf = on_battlefield(&engine, PlayerId::new(0), tavern_ruffian()).expect("in play");
    let def = baylee_cards::by_index(tavern_ruffian()).expect("a card in the pool");
    let state = seat0(&mut engine);
    state.day_night = None;
    state.switch_face(wolf, def, 1);
    settle(&mut engine);
    assert_eq!(engine.state().day_night, Some(DayNight::Night));
    assert_eq!(face_of(&engine, wolf), 1, "night leaves the back face up");
}

/// CR 702.145c: front face up, daybound, and it is night — the permanent
/// turns over, immediately and without using the stack.
#[test]
fn night_turns_a_daybound_permanent_over() {
    let mut engine = Duel::new(912, plains())
        .battlefield(0, &[tavern_ruffian()])
        .start();
    keep_mulligans(&mut engine);
    let wolf = on_battlefield(&engine, PlayerId::new(0), tavern_ruffian()).expect("in play");
    assert_eq!(pt(&engine, wolf), (2, 5));

    seat0(&mut engine).become_night();
    settle(&mut engine);
    assert_eq!(face_of(&engine, wolf), 1);
    assert_eq!(pt(&engine, wolf), (6, 5), "Tavern Smasher is a 6/5");
    let turned = engine
        .journal()
        .entries()
        .iter()
        .filter(|e| matches!(e.event, GameEvent::Transformed { object, face: 1 } if object == wolf))
        .count();
    assert_eq!(turned, 1, "a transform is one journal entry");
}

/// CR 702.145f, the mirror: back face up, nightbound, and it is day.
///
/// Written as a round trip rather than as a second setup, because the pair
/// of rules is what a werewolf actually does — and a transform that only
/// worked in one direction would pass a one-way test twice.
#[test]
fn day_turns_a_nightbound_permanent_back() {
    let mut engine = Duel::new(913, plains())
        .battlefield(0, &[tavern_ruffian()])
        .start();
    keep_mulligans(&mut engine);
    let wolf = on_battlefield(&engine, PlayerId::new(0), tavern_ruffian()).expect("in play");
    seat0(&mut engine).become_night();
    settle(&mut engine);
    assert_eq!(face_of(&engine, wolf), 1);

    seat0(&mut engine).become_day();
    settle(&mut engine);
    assert_eq!(face_of(&engine, wolf), 0);
    assert_eq!(pt(&engine, wolf), (2, 5));
}

/// CR 105.2c: the back face has no mana cost and takes its color from the
/// indicator printed on it. Nothing else says Dire-Strain Brawler is green,
/// so without the indicator every werewolf stopped being a color the moment
/// it turned over — and "target green creature" would have missed it.
#[test]
fn a_transformed_werewolf_keeps_its_color() {
    let mut engine = Duel::new(914, plains())
        .battlefield(0, &[tireless_hauler()])
        .start();
    keep_mulligans(&mut engine);
    let wolf = on_battlefield(&engine, PlayerId::new(0), tireless_hauler()).expect("in play");
    let green = baylee_core::color::ColorSet::from_slice(&[baylee_core::color::Color::Green]);
    assert_eq!(
        engine
            .state()
            .object(wolf)
            .unwrap()
            .characteristics()
            .colors,
        green
    );

    seat0(&mut engine).become_night();
    settle(&mut engine);
    assert_eq!(face_of(&engine, wolf), 1);
    assert_eq!(
        engine
            .state()
            .object(wolf)
            .unwrap()
            .characteristics()
            .colors,
        green,
        "the back face lost its color with its mana cost"
    );
    assert!(
        engine
            .state()
            .object(wolf)
            .unwrap()
            .characteristics()
            .keywords
            .contains(baylee_cards_dsl::KeywordSet::VIGILANCE),
        "Dire-Strain Brawler prints vigilance too, and a back face inherits nothing"
    );
}

/// CR 702.145b: if it is night when a daybound card would enter, it enters
/// transformed. Cast for real rather than placed on the battlefield by the
/// harness, because the clause is about *entering* and the setup path never
/// enters anything.
#[test]
fn a_daybound_card_cast_at_night_enters_transformed() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(915, plains())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .hand(0, &[tavern_ruffian()])
        .start();
    keep_mulligans(&mut engine);
    // Nothing with daybound is in play yet, so the game still has no
    // designation and CR 702.145d has nothing to fire on.
    assert_eq!(engine.state().day_night, None);
    seat0(&mut engine).become_night();

    reach_main_phase(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority in the main phase")
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .expect("a Mountain taps for {R}");
    }
    let card = engine.state().zones.list(ZoneLocation::Hand(p0))[0];
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("four Mountains pay {3}{R}");
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, tavern_ruffian()).is_some()
    });

    let wolf = on_battlefield(&engine, p0, tavern_ruffian()).expect("it resolved");
    assert_eq!(
        face_of(&engine, wolf),
        1,
        "it entered front face up at night"
    );
    assert_eq!(pt(&engine, wolf), (6, 5));
}

/// CR 701.27c: only a permanent represented by a transforming double-faced
/// card can transform. A clone of a werewolf is the case that finds this —
/// it carries the copied daybound over a card definition with one face, so
/// turning it over would rebuild its base from a face that is not there and
/// wipe the copy.
///
/// The important half of the assertion is that the engine *answers*. The
/// guard has to skip such a permanent without reporting that anything
/// changed, or the fixpoint would find work to do on it forever and the
/// game would never hand priority back.
#[test]
fn a_daybound_permanent_with_one_face_is_left_alone() {
    let mut engine = Duel::new(916, plains()).battlefield(0, &[plains()]).start();
    keep_mulligans(&mut engine);
    let land = on_battlefield(&engine, PlayerId::new(0), plains()).expect("in play");
    let state = seat0(&mut engine);
    state.day_night = Some(DayNight::Night);
    state.object_mut(land).expect("in play").base_mut().keywords =
        baylee_cards_dsl::KeywordSet::DAYBOUND;
    state.invalidate_projections();

    settle(&mut engine);
    assert_eq!(
        face_of(&engine, land),
        0,
        "a one-faced card cannot turn over"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "the fixpoint never settled: {:?}",
        engine.pending()
    );
}

/// The two mechanics joined up: a full turn in which the active player casts
/// nothing turns day into night (CR 502.2), and the werewolf on the table
/// turns over because of it (CR 702.145c). Neither half was told about the
/// other.
#[test]
fn a_quiet_turn_turns_the_werewolf_over_by_itself() {
    let mut engine = Duel::new(917, plains())
        .battlefield(0, &[tavern_ruffian()])
        .start();
    keep_mulligans(&mut engine);
    let wolf = on_battlefield(&engine, PlayerId::new(0), tavern_ruffian()).expect("in play");
    assert_eq!(engine.state().day_night, Some(DayNight::Day));
    assert_eq!(face_of(&engine, wolf), 0);

    pass_until(&mut engine, |e| e.state().turn.number >= 2);
    assert_eq!(engine.state().day_night, Some(DayNight::Night));
    assert_eq!(face_of(&engine, wolf), 1);
    assert_eq!(pt(&engine, wolf), (6, 5));
}
