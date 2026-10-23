//! `cards/creatures/mv_2/revered_unicorn.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_cards_dsl::counters;
use baylee_core::mana::ManaColor;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn revered_unicorn() -> CardIndex {
    card_index("18a8b50b-2328-459b-979d-04c4c769e569")
}

fn lightning_bolt() -> CardIndex {
    card_index("4457ed35-7c10-48c8-9776-456485fdf070")
}

/// The Unicorn with two Plains and a Mountain, hand `bolt` if asked for, run to
/// the first question of P0's upkeep.
fn table(with_bolt: bool) -> Engine<RegistryLookup> {
    let hand: &[CardIndex] = if with_bolt { &[lightning_bolt()] } else { &[] };
    let mut e = Duel::new(SEED, plains())
        .battlefield(0, &[revered_unicorn(), plains(), plains(), mountain()])
        .hand(0, hand)
        .start();
    keep_mulligans(&mut e);
    e
}

/// The tax question of P0's upkeep on `turn`, and the generic mana it names.
fn upkeep_price(e: &mut Engine<RegistryLookup>, turn: u32) -> u16 {
    pass_until(e, |e| {
        e.state().turn.number == turn && matches!(e.pending(), Pending::YesNo { .. })
    });
    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayTax { mana },
        ..
    } = e.pending().clone()
    else {
        panic!("the upkeep asks to pay mana: {:?}", e.pending())
    };
    assert_eq!(player, P0);
    mana
}

/// Says yes and pays by tapping one untapped Plains.
#[track_caller]
fn pay_with_a_plains(e: &mut Engine<RegistryLookup>) {
    let plains_id = lk::obj(e, P0, plains());
    e.apply(P0, PlayerAction::YesNo(true)).unwrap();
    e.apply(
        P0,
        PlayerAction::PayWindow {
            steps: vec![PlayerAction::ActivateManaAbility { source: plains_id }],
        },
    )
    .expect("the Plains pays");
}

/// Revered Unicorn (18a8b50b-2328-459b-979d-04c4c769e569) — "Cumulative upkeep
/// {1}": the first upkeep puts an age counter and asks {1}; paid, the Unicorn
/// stays and the opponent's upkeep adds nothing; the next of its controller's
/// asks {2}; unpaid, it is sacrificed and "when this creature leaves the
/// battlefield, you gain life equal to the number of age counters on it": 2,
/// to its controller only.
#[test]
fn revered_unicorn_gains_life_equal_to_its_age_counters_when_sacrificed() {
    let mut e = table(false);
    let unicorn = on_battlefield(&e, P0, revered_unicorn()).expect("the Unicorn is out");

    assert_eq!(upkeep_price(&mut e, 1), 1, "one age counter, {{1}}");
    assert_eq!(counters_on(&e, unicorn, counters::AGE), 1);
    pay_with_a_plains(&mut e);
    pass_until(&mut e, stack_is_empty);
    assert!(
        on_battlefield(&e, P0, revered_unicorn()).is_some(),
        "paid, so it stays"
    );
    assert_eq!(lk::life(&e, P0), 20, "nothing left the battlefield yet");

    reach_their_main_phase(&mut e, P1);
    assert_eq!(
        counters_on(&e, unicorn, counters::AGE),
        1,
        "P1's upkeep is not yours"
    );

    assert_eq!(upkeep_price(&mut e, 3), 2, "two age counters, {{1}} each");
    assert_eq!(counters_on(&e, unicorn, counters::AGE), 2);
    e.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(
        in_graveyard(&e, P0, revered_unicorn()).is_some(),
        "unpaid: sacrificed"
    );
    assert_eq!(lk::life(&e, P0), 22, "two age counters, two life");
    assert_eq!(lk::life(&e, P1), 20, "the opponent gains nothing");
}

/// Revered Unicorn — declined at the very first upkeep it is sacrificed with
/// one age counter on it: one life.
#[test]
fn revered_unicorn_gains_one_life_when_its_first_upkeep_goes_unpaid() {
    let mut e = table(false);

    assert_eq!(upkeep_price(&mut e, 1), 1);
    e.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(in_graveyard(&e, P0, revered_unicorn()).is_some());
    assert_eq!(lk::life(&e, P0), 21);
}

/// Revered Unicorn — the life trigger is for any way of leaving, not only the
/// failed upkeep: after one paid upkeep a Lightning Bolt kills the 2/3 and the
/// one age counter on it is one life.
#[test]
fn revered_unicorn_gains_life_when_it_dies_to_damage_too() {
    let mut e = table(true);
    let unicorn = on_battlefield(&e, P0, revered_unicorn()).expect("the Unicorn is out");

    assert_eq!(upkeep_price(&mut e, 1), 1);
    pay_with_a_plains(&mut e);
    pass_until(&mut e, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain)
            && matches!(e.pending(), Pending::Priority { .. })
    });

    lk::float(&mut e, P0, &[(ManaColor::Red, 1)]);
    cast_with_floating(&mut e, P0, lightning_bolt());
    aim_at(&mut e, P0, unicorn);
    pass_until(&mut e, stack_is_empty);

    assert!(
        in_graveyard(&e, P0, revered_unicorn()).is_some(),
        "3 damage kills a 2/3"
    );
    assert_eq!(lk::life(&e, P0), 21, "one age counter, one life");
}
