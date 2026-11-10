//! `cards/creatures/mv_3/capashen_templar.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Capashen Templar is `{2}{W}` for a 2/2 Human Knight with one printed line:
/// "{W}: This creature gets +0/+1 until end of turn." Two readings hold the
/// number down from either side — a `+1/+1` or a `+0/+2` in the card file
/// would both satisfy a test that only looked at the toughness — so the
/// assertion is on the pair, `(2, 2)` to `(2, 3)` with the power untouched.
/// The Llanowar Elves beside it is a creature under the same seat that "This
/// creature" must not reach, and it is kept out of the mana so that the four
/// white in the pool are the four tapped Plains: three pay the creature and
/// the last one pays the activation, which is what makes the empty pool
/// afterwards a claim about the printed `{W}` rather than about a free
/// ability. The turn boundary is played out because "until end of turn" is
/// the half of the sentence a permanent pump would quietly keep.
#[test]
fn capashen_templar_pays_white_for_one_toughness_on_itself_and_only_until_the_turn_ends() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[plains(), plains(), plains(), plains(), llanowar_elves()],
        )
        .hand(0, &[capashen_templar()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The Elf is the bystander and is named as the printing kept back: a
    // source tapped for mana is a creature whose status has already changed
    // for a reason of its own, and its own {G} would make "four white" a
    // claim about a pool nothing on the board accounted for.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four tapped Plains, four white, and the Elf contributed nothing"
    );
    cast_with_floating(&mut engine, p0, capashen_templar());
    pass_until(&mut engine, stack_is_empty);

    let templar = on_battlefield(&engine, p0, capashen_templar()).expect("the Templar resolved");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is still out");
    assert_eq!(pt(&engine, templar), (2, 2), "the printed 2/2 body");
    assert_eq!(
        pt(&engine, elves),
        (1, 1),
        "and the bystander it may not touch"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "{{2}}{{W}} is three of the four, so one white is left for the ability"
    );

    // Ability 0 is the only line the card prints, and it targets nothing: the
    // pump is aimed at its own source by `Filter::This`, so no target
    // question stands between the activation and its payment.
    activate(&mut engine, p0, capashen_templar(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, templar),
        (2, 3),
        "+0/+1 on this creature — a (3, 3) would mean the power was read too"
    );
    assert_eq!(
        pt(&engine, elves),
        (1, 1),
        "\"This creature\" is not \"creatures you control\": the Elf is untouched"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{W}} came out of the pool, so the price is real"
    );

    // The effect's duration. The whole of p0's turn is played out — combat
    // asks for attackers and blockers on the way and answers both empty —
    // because a pump that had no duration would still be standing here.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        pt(&engine, templar),
        (2, 2),
        "\"until end of turn\": the toughness is the printed one again"
    );
}
