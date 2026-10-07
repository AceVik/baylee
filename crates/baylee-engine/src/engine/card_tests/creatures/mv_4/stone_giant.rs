//! `cards/creatures/mv_4/stone_giant.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Stone Giant targets only your creature with toughness strictly below its
/// power, then destroys that creature at the next end step (CR 603.7).
#[test]
fn alpha_eval_stone_giant_throws_only_a_smaller_friendly_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let giant = card_index("0b8e3f9b-a4da-49a3-8545-ce7a265e5856");
    let equal = card_index("342199e0-15b6-4824-83da-25caef2592b3");
    let mut engine = Duel::new(1006, forest())
        .battlefield(0, &[giant, llanowar_elves(), equal])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elf = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    activate(&mut engine, p0, giant, 0);
    let options = aim_at(&mut engine, p0, elf);
    assert_eq!(
        options,
        vec![elf],
        "not their Elf, the Giant, or equal toughness"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, elf).contains(KeywordSet::FLYING));
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::End
    });
    assert!(!stack_is_empty(&engine));
    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_some());
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, llanowar_elves()).is_some());
    assert!(on_battlefield(&engine, p1, llanowar_elves()).is_some());
    assert!(on_battlefield(&engine, p0, giant).is_some());
}
