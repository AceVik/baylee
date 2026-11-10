//! `cards/enchantments/mv_2/night_soil.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn soil_card() -> CardIndex {
    card_index("3165fe8f-52d7-40f7-bb14-8f4300a564e6")
}
fn bears_card() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}
fn hill_giant_card() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn game() -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, swamp())
        .battlefield(0, &[soil_card(), forest(), bears_card(), hill_giant_card()])
        .battlefield(1, &[bears_card(), hill_giant_card()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    e
}

fn saprolings(e: &Engine<RegistryLookup>) -> Vec<ObjectId> {
    e.state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            let o = e.state().object(id).expect("on the battlefield");
            !o.is_card() && o.controller == P0
        })
        .collect()
}

/// Night Soil (3165fe8f-52d7-40f7-bb14-8f4300a564e6) — "{1}, Exile two
/// creature cards from a single graveyard: Create a 1/1 green Saproling
/// creature token." Two creature cards of one graveyard are exiled and a
/// Saproling arrives.
#[test]
fn night_soil_exiles_two_creature_cards_from_one_graveyard_for_a_saproling() {
    let mut e = game();
    let mine = on_battlefield(&e, P0, bears_card()).expect("my Bears");
    let giant = on_battlefield(&e, P0, hill_giant_card()).expect("my Giant");
    bury(&mut e, &[mine, giant]);
    e.refresh_offer();
    assert_eq!(e.state().zones.list(ZoneLocation::Graveyard(P0)).len(), 2);

    tap_all_mana(&mut e, P0);
    activate(&mut e, P0, soil_card(), 0);
    let Pending::ChooseCards {
        player, options, ..
    } = e.pending().clone()
    else {
        panic!("the exile cost is a choice, got {:?}", e.pending())
    };
    assert_eq!(player, P0);
    assert!(options.contains(&mine) && options.contains(&giant));
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![mine, giant],
        },
    )
    .expect("two creature cards of one graveyard are a legal cost");
    pass_until(&mut e, stack_is_empty);

    assert!(
        e.state().zones.list(ZoneLocation::Graveyard(P0)).is_empty(),
        "both cards left the graveyard"
    );
    for id in [mine, giant] {
        assert_eq!(
            e.state().object(id).map(|o| o.zone),
            Some(Zone::Exile),
            "exiled as the cost"
        );
    }
    let made = saprolings(&e);
    assert_eq!(made.len(), 1, "one Saproling");
    assert_eq!(pt(&e, made[0]), (1, 1));
}

/// Night Soil — "from a single graveyard": the cards of two different
/// graveyards do not add up, so with one creature card in each the ability
/// cannot be activated.
#[test]
fn night_soil_cannot_mix_graveyards() {
    let mut e = game();
    let mine = on_battlefield(&e, P0, bears_card()).expect("my Bears");
    let theirs = on_battlefield(&e, P1, hill_giant_card()).expect("their Giant");
    bury(&mut e, &[mine, theirs]);
    e.refresh_offer();
    assert_eq!(e.state().zones.list(ZoneLocation::Graveyard(P0)).len(), 1);
    assert_eq!(e.state().zones.list(ZoneLocation::Graveyard(P1)).len(), 1);

    tap_all_mana(&mut e, P0);
    let soil = on_battlefield(&e, P0, soil_card()).expect("Night Soil");
    assert!(
        !priority_offer(&e).abilities.contains(&(soil, 0)),
        "one creature card in each graveyard is not two from a single one"
    );
}

/// Night Soil — it can use the opponent's graveyard too, as long as both
/// cards come from it.
#[test]
fn night_soil_may_exile_from_an_opponents_graveyard() {
    let mut e = game();
    let b = on_battlefield(&e, P1, bears_card()).expect("their Bears");
    let g = on_battlefield(&e, P1, hill_giant_card()).expect("their Giant");
    bury(&mut e, &[b, g]);
    e.refresh_offer();

    tap_all_mana(&mut e, P0);
    activate(&mut e, P0, soil_card(), 0);
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![b, g],
        },
    )
    .expect("both cards are in P1's graveyard");
    pass_until(&mut e, stack_is_empty);
    assert!(e.state().zones.list(ZoneLocation::Graveyard(P1)).is_empty());
    assert_eq!(saprolings(&e).len(), 1);
}

/// Night Soil — needs two cards: a single creature card is not enough.
#[test]
fn night_soil_needs_two_creature_cards() {
    let mut e = game();
    let mine = on_battlefield(&e, P0, bears_card()).expect("my Bears");
    bury(&mut e, &[mine]);
    e.refresh_offer();
    tap_all_mana(&mut e, P0);
    let soil = on_battlefield(&e, P0, soil_card()).expect("Night Soil");
    assert!(!priority_offer(&e).abilities.contains(&(soil, 0)));
}

mod selection;
