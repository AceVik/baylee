//! `cards/creatures/mv_3/fog_elemental.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn fog_elemental() -> CardIndex {
    card_index("b304fcc0-899e-433f-b6df-5cea504dc4ba")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

/// Fog Elemental (b304fcc0-899e-433f-b6df-5cea504dc4ba) — "When this creature
/// attacks or blocks, sacrifice it at end of combat." Attacking: its four
/// damage is dealt first, and it is sacrificed only when the end of combat
/// step begins.
#[test]
fn fog_elemental_attacks_deals_its_damage_then_is_sacrificed_at_end_of_combat() {
    let mut e = lk::game(SEED, &[fog_elemental()], &[], &[], &[]);
    let fog = lk::obj(&e, P0, fog_elemental());

    lk::attack(&mut e, &[fog]);
    lk::block(&mut e, &[]);
    lk::to_step(&mut e, P0, Step::CombatDamage);
    assert_eq!(lk::life(&e, P1), 16, "the damage was dealt");
    assert!(
        on_battlefield(&e, P0, fog_elemental()).is_some(),
        "and the Elemental is still there until the end of combat"
    );

    lk::to_step(&mut e, P0, Step::CombatEnd);
    lk::settle(&mut e);
    assert!(in_graveyard(&e, P0, fog_elemental()).is_some());
    assert!(on_battlefield(&e, P0, fog_elemental()).is_none());
}

/// Fog Elemental — blocking counts as well: it kills the attacker it blocks
/// and is sacrificed at end of combat.
#[test]
fn fog_elemental_blocks_kills_the_attacker_then_is_sacrificed() {
    let mut e = lk::game(SEED, &[fog_elemental()], &[savannah_lions()], &[], &[]);
    let fog = lk::obj(&e, P0, fog_elemental());
    let lions = lk::obj(&e, P1, savannah_lions());

    pass_until(&mut e, |e| {
        e.state().turn.active == P1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    lk::declare(&mut e, &[lions]);
    lk::block(&mut e, &[(fog, lions)]);
    lk::to_step(&mut e, P1, Step::CombatDamage);
    assert!(
        in_graveyard(&e, P1, savannah_lions()).is_some(),
        "four damage killed the Lions"
    );
    assert!(on_battlefield(&e, P0, fog_elemental()).is_some());

    lk::to_step(&mut e, P1, Step::CombatEnd);
    lk::settle(&mut e);
    assert!(in_graveyard(&e, P0, fog_elemental()).is_some());
}

/// Fog Elemental — an Elemental that neither attacks nor blocks is never
/// sacrificed: it sits out its own turn and the opponent's attack.
#[test]
fn fog_elemental_that_stays_out_of_combat_stays() {
    let mut e = lk::game(SEED, &[fog_elemental()], &[savannah_lions()], &[], &[]);
    let lions = lk::obj(&e, P1, savannah_lions());

    pass_until(&mut e, |e| {
        e.state().turn.active == P1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    lk::declare(&mut e, &[lions]);
    lk::block(&mut e, &[]);
    pass_until(&mut e, |e| {
        e.state().turn.active == P1 && e.state().turn.phase == Phase::SecondMain
    });
    assert_eq!(lk::life(&e, P0), 18, "the Lions went unblocked");
    assert!(
        on_battlefield(&e, P0, fog_elemental()).is_some(),
        "no attack and no block, so no sacrifice"
    );
}
