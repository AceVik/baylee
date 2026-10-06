//! `cards/creatures/mv_6/scaled_hulk.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "ce4570eb-f637-45d1-afbc-f11357c09bf9"

/// Scaled Hulk is a {5}{G} 4/4 Spirit whose whole printed text is "Whenever
/// you cast a Spirit or Arcane spell, this creature gets +2/+2 until end of
/// turn." The scenario casts two spells in one main phase because the
/// disjunction needs both sides on the board: a Llanowar Elves, an Elf Druid
/// that must leave the Hulk at its printed 4/4, and a second Scaled Hulk,
/// which is a Spirit and must turn the first one into a 6/6 — the resolving
/// copy is not on the battlefield while it is being cast (CR 113.6), so only
/// the seated one's ability can trigger. The second Hulk is read as well, so
/// "this creature" cannot pass for a board-wide pump, and the turn afterwards
/// is walked because "until end of turn" is half of what the card says.
#[test]
fn scaled_hulk_grows_for_a_spirit_spell_and_only_for_the_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                scaled_hulk(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[llanowar_elves(), scaled_hulk()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let hulk = on_battlefield(&engine, p0, scaled_hulk()).expect("the Hulk is seated");
    assert_eq!(pt(&engine, hulk), (4, 4), "the body the card prints");

    // Seven Forests into the pool: the Hulk prints no mana ability of its own,
    // so this is every source on the board.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        7,
        "seven tapped Forests, seven green"
    );

    // The negative half: a creature spell that is neither a Spirit nor an
    // Arcane spell, so the printed sentence has nothing to trigger on.
    cast_with_floating(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, hulk),
        (4, 4),
        "an Elf Druid spell is neither a Spirit nor an Arcane spell"
    );

    // The positive half, and the {5}{G} the pool has left is exactly the
    // second Hulk's cost.
    cast_with_floating(&mut engine, p0, scaled_hulk());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, hulk),
        (6, 6),
        "\"Whenever you cast a Spirit … spell, this creature gets +2/+2\""
    );
    let hulks = all_on_battlefield(&engine, p0, scaled_hulk());
    assert_eq!(hulks.len(), 2, "the second copy resolved beside the first");
    let other = *hulks
        .iter()
        .find(|id| **id != hulk)
        .expect("one of the two is the seated Hulk");
    assert_eq!(
        pt(&engine, other),
        (4, 4),
        "\"this creature\": the pump lands on the source and not on every \
         creature its controller has"
    );

    // "until end of turn" — a turn later the +2/+2 is gone from a creature
    // that is still standing on the battlefield.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, hulk),
        (4, 4),
        "the pump lasted the turn it was made in and no longer"
    );
    assert_eq!(
        pt(&engine, other),
        (4, 4),
        "and the copy that was never pumped is where it was"
    );
}
