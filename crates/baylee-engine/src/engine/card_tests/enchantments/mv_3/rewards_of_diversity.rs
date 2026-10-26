//! Rewards of Diversity — {2}{W} — Enchantment
//! Oracle: Whenever an opponent casts a multicolored spell, you gain 4 life.

use super::*;
use crate::engine::card_tests::legends_kit::life;
use baylee_core::generated::index;

#[test]
fn rewards_of_diversity_gains_four_life_when_opponent_casts_multicolored_spell() {
    let rewards = card_index("064c5a14-c2f2-4b7a-8e0b-01b9995e8f89");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[rewards])
        .hand(1, &[index::GRIZZLY_BEARS, index::METEOR_STORM])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    cast_from_hand(&mut engine, p0, rewards);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, rewards).is_some());

    reach_their_main_phase(&mut engine, p1);
    crate::engine::card_tests::legends_kit::float(
        &mut engine,
        p1,
        &[(ManaColor::Colorless, 1), (ManaColor::Green, 1)],
    );
    cast_with_floating(&mut engine, p1, index::GRIZZLY_BEARS);
    pass_until(&mut engine, stack_is_empty);
    // Monocolored spell does not trigger Rewards of Diversity.
    assert_eq!(life(&engine, p0), 20);

    crate::engine::card_tests::legends_kit::float(
        &mut engine,
        p1,
        &[(ManaColor::Red, 1), (ManaColor::Green, 1)],
    );
    cast_with_floating(&mut engine, p1, index::METEOR_STORM);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life(&engine, p0), 24);
}
