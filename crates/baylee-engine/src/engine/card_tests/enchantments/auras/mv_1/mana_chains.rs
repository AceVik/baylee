//! `cards/enchantments/auras/mv_1/mana_chains.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::aura_kit as kit;
use crate::engine::card_tests::legends_kit as lk;
use baylee_cards_dsl::counters;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn mana_chains() -> CardIndex {
    card_index("340d27c6-2e8e-472a-85f5-9b36d9cc3e42")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// The tax question of the upkeep on `turn`: who is asked, and the generic
/// mana it names.
fn upkeep_question(e: &mut Engine<RegistryLookup>, turn: u32) -> (PlayerId, u16) {
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
    (player, mana)
}

/// Mana Chains (340d27c6-2e8e-472a-85f5-9b36d9cc3e42) — "Enchanted creature
/// has 'Cumulative upkeep {1}.'" On its controller's own Bears: the next
/// upkeep asks {1} (one age counter), paid with an Island the Bears live;
/// the one after asks {2} (two counters), declined, the Bears are
/// sacrificed and the Aura follows them to the graveyard.
#[test]
fn mana_chains_asks_one_more_each_upkeep_of_the_enchanted_creatures_controller() {
    let mut e = lk::game(
        SEED,
        &[grizzly_bears(), island(), island(), island()],
        &[],
        &[mana_chains()],
        &[],
    );
    let bears = lk::obj(&e, P0, grizzly_bears());
    kit::cast_aura(&mut e, P0, mana_chains(), bears, &[(ManaColor::Blue, 1)]);
    assert_eq!(counters_on(&e, bears, counters::AGE), 0, "no counter yet");

    assert_eq!(
        upkeep_question(&mut e, 3),
        (P0, 1),
        "one age counter, {{1}}"
    );
    assert_eq!(counters_on(&e, bears, counters::AGE), 1);
    let island_id = lk::obj(&e, P0, island());
    e.apply(P0, PlayerAction::YesNo(true)).unwrap();
    e.apply(
        P0,
        PlayerAction::PayWindow {
            steps: vec![PlayerAction::ActivateManaAbility { source: island_id }],
        },
    )
    .expect("an Island pays {1}");
    pass_until(&mut e, stack_is_empty);
    assert!(
        on_battlefield(&e, P0, grizzly_bears()).is_some(),
        "paid, so it stays"
    );

    assert_eq!(
        upkeep_question(&mut e, 5),
        (P0, 2),
        "two age counters, {{2}}"
    );
    e.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(
        in_graveyard(&e, P0, grizzly_bears()).is_some(),
        "unpaid: sacrificed"
    );
    assert!(
        in_graveyard(&e, P0, mana_chains()).is_some(),
        "and the Aura with it"
    );
}

/// Mana Chains — "its controller's upkeep, that player": enchanting the
/// OPPONENT's Bears, the question goes to the opponent at their upkeep, the
/// Aura's caster is asked nothing on theirs, and when the opponent declines
/// it is their Bears that are sacrificed.
#[test]
fn mana_chains_on_an_opposing_creature_taxes_its_controller_not_the_caster() {
    let mut e = lk::game(SEED, &[], &[grizzly_bears()], &[mana_chains()], &[]);
    let theirs = lk::obj(&e, P1, grizzly_bears());
    kit::cast_aura(&mut e, P0, mana_chains(), theirs, &[(ManaColor::Blue, 1)]);

    let (asked, mana) = upkeep_question(&mut e, 2);
    assert_eq!(
        (asked, mana),
        (P1, 1),
        "the creature's controller pays, on their upkeep"
    );
    e.apply(P1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(
        in_graveyard(&e, P1, grizzly_bears()).is_some(),
        "their Bears were sacrificed"
    );
    assert!(
        in_graveyard(&e, P0, mana_chains()).is_some(),
        "the Aura falls off with them"
    );
}
