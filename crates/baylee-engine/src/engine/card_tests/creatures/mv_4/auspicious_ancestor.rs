//! `cards/creatures/mv_4/auspicious_ancestor.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{AUSPICIOUS_ANCESTOR, DARK_BANISHING, SAVANNAH_LIONS};

#[test]
fn auspicious_ancestor_gains_life_on_white_spell_and_on_death() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[SAVANNAH_LIONS, DARK_BANISHING])
        .battlefield(
            0,
            &[
                AUSPICIOUS_ANCESTOR,
                plains(),
                plains(),
                swamp(),
                plains(),
                plains(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ancestor = on_battlefield(&engine, p0, AUSPICIOUS_ANCESTOR).unwrap();
    let lions = in_hand(&engine, p0, SAVANNAH_LIONS).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: lions })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 21);

    // Destroy Auspicious Ancestor to trigger its death ability (+3 life).
    let banishing = in_hand(&engine, p0, DARK_BANISHING).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: banishing })
        .unwrap();
    aim_at(&mut engine, p0, ancestor);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().players[0].life, 24);
}
