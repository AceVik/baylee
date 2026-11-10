//! `cards/creatures/mv_2/ironclaw_orcs.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Ironclaw Orcs — "This creature can't block creatures with power 2 or
/// greater." A power-2 attacker slips past it; a power-1 attacker does not.
#[test]
fn ironclaw_orcs_cannot_block_a_creature_with_power_two_or_more() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[pearled_unicorn(), llanowar_elves()])
        .battlefield(1, &[ironclaw_orcs()])
        .start();
    keep_mulligans(&mut engine);
    let strong = on_battlefield(&engine, p0, pearled_unicorn()).expect("power 2");
    let weak = on_battlefield(&engine, p0, llanowar_elves()).expect("power 1");
    let orcs = on_battlefield(&engine, p1, ironclaw_orcs()).expect("seated");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { player, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for exactly this")
    };
    engine
        .apply(
            player,
            PlayerAction::DeclareAttackers {
                attackers: vec![(strong, Defender::Player(p1)), (weak, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        panic!("expected the block offer, got {:?}", engine.pending())
    };
    let pairing = blockers
        .iter()
        .find(|b| b.blocker == orcs)
        .unwrap_or_else(|| panic!("the Orcs could still block the weak attacker: {blockers:?}"));
    assert!(
        !pairing.attackers.contains(&strong),
        "power 2 or greater: not blockable by the Orcs: {pairing:?}"
    );
    assert!(
        pairing.attackers.contains(&weak),
        "power 1: the Orcs may still block it: {pairing:?}"
    );
}
