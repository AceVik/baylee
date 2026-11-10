//! `cards/artifacts/mv_1/cursed_scroll.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn cursed_scroll() -> CardIndex {
    card_index("514752ea-d453-41d4-97cf-b1dcdb4b4a08")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// p0 has the Scroll and three Mountains, `held` as the whole hand, and their
/// first main phase.
fn scroll_over_a_hand(held: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[cursed_scroll(), mountain(), mountain(), mountain()])
        .hand(0, held)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        held.len(),
        "the hand is exactly what the scenario dealt"
    );
    engine
}

/// Pays {3} and taps, aims the 2 damage at p1, names `name` on resolution and
/// lets the ability finish.
#[track_caller]
fn fire_naming(engine: &mut Engine<RegistryLookup>, name: CardIndex) {
    tap_all_mana(engine, P0);
    activate(engine, P0, cursed_scroll(), 0);
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseCardName { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::ChooseCardName {
                card: name,
                face: 0,
            },
        )
        .unwrap();
    pass_until(engine, stack_is_empty);
}

/// Cursed Scroll (514752ea-d453-41d4-97cf-b1dcdb4b4a08) — {1} Artifact:
/// "{3}, {T}: Choose a card name, then reveal a card at random from your hand. If that card has the chosen name, this artifact deals 2 damage to any target."
#[test]
fn cursed_scroll_deals_two_damage_when_the_revealed_card_has_the_chosen_name() {
    let mut engine = scroll_over_a_hand(&[grizzly_bears()]);

    fire_naming(&mut engine, grizzly_bears());

    assert_eq!(engine.state().players[1].life, 18);
    let scroll = on_battlefield(&engine, P0, cursed_scroll()).unwrap();
    assert!(is_tapped(&engine, scroll), "the {{T}} was paid");
}

#[test]
fn cursed_scroll_deals_no_damage_when_the_revealed_card_has_another_name() {
    let mut engine = scroll_over_a_hand(&[grizzly_bears()]);

    fire_naming(&mut engine, hill_giant());

    assert_eq!(
        engine.state().players[1].life,
        20,
        "the revealed Grizzly Bears is not the named Hill Giant"
    );
    assert!(
        in_hand(&engine, P0, grizzly_bears()).is_some(),
        "revealing does not move the card"
    );
}

#[test]
fn cursed_scroll_with_an_empty_hand_reveals_nothing_and_deals_no_damage() {
    let mut engine = scroll_over_a_hand(&[]);

    fire_naming(&mut engine, grizzly_bears());

    assert_eq!(engine.state().players[1].life, 20);
}
