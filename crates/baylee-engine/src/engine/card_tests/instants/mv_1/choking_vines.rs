//! `cards/instants/mv_1/choking_vines.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn choking_vines() -> CardIndex {
    card_index("3b6b565e-c0b5-48f9-b3e0-4d5d4b065402")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

fn savannah_lions() -> CardIndex {
    card_index("60ba93eb-39e6-4af2-9c66-cd38f72daff2")
}

/// Whether the Vines may be cast now, mana already made or still to be made.
fn may_cast(e: &Engine<RegistryLookup>, seat: PlayerId) -> bool {
    let Some(id) = in_hand(e, seat, choking_vines()) else {
        return false;
    };
    let Pending::Priority { legal, .. } = e.pending() else {
        return false;
    };
    legal.castable.contains(&id) || legal.payable.contains(&id)
}

/// P1 attacks P0 with a Hill Giant (3/3) and a Savannah Lions (2/1); P0 has
/// the Vines in hand and blocks nothing. Returns at P0's priority in the
/// declare blockers step. Before the attack, the main phase refuses the cast
/// with the mana to pay for it floating.
fn attack_unblocked() -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let mut e = lk::game(
        SEED,
        &[],
        &[hill_giant(), savannah_lions()],
        &[choking_vines()],
        &[],
    );
    let giant = lk::obj(&e, P1, hill_giant());
    let lions = lk::obj(&e, P1, savannah_lions());

    lk::float(
        &mut e,
        P0,
        &[(ManaColor::Green, 1), (ManaColor::Colorless, 2)],
    );
    assert!(
        !may_cast(&e, P0),
        "\"only during the declare blockers step\": not in a main phase"
    );

    pass_until(&mut e, |e| {
        e.state().turn.active == P1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    lk::declare(&mut e, &[giant, lions]);
    lk::block(&mut e, &[]);
    lk::pass_once(&mut e);
    assert!(
        matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
        "P0 has priority in the declare blockers step"
    );
    lk::float(
        &mut e,
        P0,
        &[(ManaColor::Green, 1), (ManaColor::Colorless, 2)],
    );
    assert!(may_cast(&e, P0), "and now the Vines can be cast");
    (e, giant, lions)
}

/// Choking Vines (3b6b565e-c0b5-48f9-b3e0-4d5d4b065402) — "X target attacking
/// creatures become blocked. Choking Vines deals 1 damage to each of those
/// creatures." The unsupported blocked instruction is omitted. X=2 deals
/// one to each attacker: Lions die, Giant survives and still deals damage.
#[test]
fn choking_vines_x_two_damages_both_attackers_without_suppressing_combat_damage() {
    let (mut e, giant, lions) = attack_unblocked();
    lk::announce(&mut e, P0, choking_vines(), 0, 2, &[giant, lions], &[]);
    lk::settle(&mut e);
    assert_eq!(lk::damage_on(&e, giant), 1, "one damage to the Giant");
    assert!(
        in_graveyard(&e, P1, savannah_lions()).is_some(),
        "one damage kills the 2/1"
    );

    lk::to_step(&mut e, P1, Step::CombatEnd);
    assert_eq!(lk::life(&e, P0), 17, "the surviving Giant still attacks");
}

/// Choking Vines — only the chosen attackers take damage: X=1 on the Giant
/// leaves the Lions undamaged. Both attackers still deal combat damage.
#[test]
fn choking_vines_x_one_damages_only_the_chosen_attacker() {
    let (mut e, giant, lions) = attack_unblocked();
    lk::announce(&mut e, P0, choking_vines(), 0, 1, &[giant], &[]);
    lk::settle(&mut e);
    assert_eq!(lk::damage_on(&e, giant), 1);
    assert_eq!(lk::damage_on(&e, lions), 0, "not a target, not damaged");

    lk::to_step(&mut e, P1, Step::CombatEnd);
    assert_eq!(
        lk::life(&e, P0),
        15,
        "neither attacker has its combat damage suppressed"
    );
}
