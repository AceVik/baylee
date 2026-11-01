//! `cards/enchantments/mv_4/pendrell_mists.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn pendrell_mists() -> CardIndex {
    card_index("b18b8415-a0b1-459d-8c6b-896381184935")
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

/// Pendrell Mists (b18b8415-a0b1-459d-8c6b-896381184935) — "All creatures have
/// 'At the beginning of your upkeep, sacrifice this creature unless you pay
/// {1}.'" The caster's Bears: asked {1} at their own upkeep, paid with an
/// Island it stays; the opponent's Bears: its controller is asked at the
/// opponent's upkeep, not the caster, and with no mana to pay it is sacrificed;
/// back on the caster's turn the price is {1} again, not {2} (nothing is
/// cumulative), and the Mists, no creature, is never asked about.
#[test]
fn pendrell_mists_taxes_every_creature_at_its_own_controllers_upkeep() {
    let mut e = lk::game_with(
        SEED,
        &[pendrell_mists(), grizzly_bears(), island()],
        &[grizzly_bears()],
        &[],
        &[],
        |e| {
            // Turn 1's upkeep question: pay with the Island.
            let island_id = on_battlefield(e, P0, island()).expect("the Island is out");
            e.apply(P0, PlayerAction::YesNo(true)).unwrap();
            e.apply(
                P0,
                PlayerAction::PayWindow {
                    steps: vec![PlayerAction::ActivateManaAbility { source: island_id }],
                },
            )
            .expect("the Island pays {1}");
        },
    );
    assert!(
        on_battlefield(&e, P0, grizzly_bears()).is_some(),
        "paid at turn 1, so it stays"
    );

    assert_eq!(
        upkeep_question(&mut e, 2),
        (P1, 1),
        "the opponent's Bears, asked of the opponent"
    );
    e.apply(P1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(
        in_graveyard(&e, P1, grizzly_bears()).is_some(),
        "unpaid: their Bears is sacrificed"
    );
    assert!(
        on_battlefield(&e, P0, grizzly_bears()).is_some(),
        "the caster's Bears is not"
    );

    assert_eq!(
        upkeep_question(&mut e, 3),
        (P0, 1),
        "{{1}} again, not cumulative"
    );
    e.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(
        in_graveyard(&e, P0, grizzly_bears()).is_some(),
        "declined: sacrificed"
    );
    assert!(
        on_battlefield(&e, P0, pendrell_mists()).is_some(),
        "the Mists is not a creature"
    );
}
