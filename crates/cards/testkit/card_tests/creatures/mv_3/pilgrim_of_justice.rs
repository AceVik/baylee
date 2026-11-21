//! `cards/creatures/mv_3/pilgrim_of_justice.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn pilgrim_of_justice() -> CardIndex {
    card_index("959b672e-d4e4-49a1-874a-5148f040fadd")
}

/// Pilgrim of Justice ({2}{W} 1/3 Human Cleric):
/// "Protection from red
/// {W}, Sacrifice this creature: The next time a red source of your choice
/// would deal damage this turn, prevent that damage."
#[test]
fn pilgrim_of_justice_has_protection_from_red_and_prevents_next_damage_from_chosen_red_source() {
    let mut e = lk::game(
        SEED,
        &[pilgrim_of_justice(), benalish_hero()],
        &[raging_goblin(), grizzly_bears()],
        &[],
        &[],
    );
    let pilgrim = lk::obj(&e, lk::P0, pilgrim_of_justice());
    let hero = lk::obj(&e, lk::P0, benalish_hero());
    let goblin = lk::obj(&e, lk::P1, raging_goblin());

    // Protection from red: Raging Goblin cannot block Pilgrim of Justice.
    let blocks = lk::attack(&mut e, &[pilgrim]);
    assert!(!lk::may_block(&blocks, goblin, pilgrim));
    lk::block(&mut e, &[]);
    lk::through_combat(&mut e);

    // On P1's turn, sacrifice Pilgrim to prevent Raging Goblin's next damage (even to Benalish Hero!).
    reach_their_main_phase(&mut e, lk::P1);
    let _ = lk::attack(&mut e, &[goblin]);
    lk::block(&mut e, &[(hero, goblin)]);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == lk::P0),
    );

    lk::float(&mut e, lk::P0, &[(ManaColor::White, 1)]);
    activate(&mut e, lk::P0, pilgrim_of_justice(), 1);
    let selected = baylee_core::ids::DamageSourceRef {
        object: goblin,
        version: e.state().object(goblin).unwrap().version,
    };
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        options, choice, ..
    } = e.pending().clone()
    else {
        unreachable!();
    };
    assert_eq!(options, vec![selected], "only red sources are offered");
    e.apply(
        lk::P0,
        PlayerAction::ChooseDamageSource {
            choice,
            source: selected,
        },
    )
    .unwrap();
    lk::through_combat(&mut e);

    assert!(in_graveyard(&e, lk::P0, pilgrim_of_justice()).is_some());
    assert!(on_battlefield(&e, lk::P0, benalish_hero()).is_some());
    assert!(on_battlefield(&e, lk::P1, raging_goblin()).is_none());
}
