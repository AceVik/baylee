//! `cards/instants/mv_3/psionic_blast.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Psionic Blast: "deals 4 damage to any target" and, on its own second
/// line, "deals 2 damage to you" — its own caster, unconditionally,
/// regardless of who or what the first line hits.
#[test]
fn psionic_blast_deals_4_to_any_target_and_2_to_its_caster() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), forest()])
        .hand(0, &[psionic_blast()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let before0 = life_of(&engine, p0);
    let before1 = life_of(&engine, p1);
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, psionic_blast());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("p1 is any target");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        life_of(&engine, p1),
        before1 - 4,
        "\"deals 4 damage to any target\""
    );
    assert_eq!(
        life_of(&engine, p0),
        before0 - 2,
        "\"deals 2 damage to you\" — its own caster"
    );
}
