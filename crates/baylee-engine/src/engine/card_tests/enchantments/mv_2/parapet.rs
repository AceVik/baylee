//! `cards/enchantments/mv_2/parapet.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;
use baylee_core::generated::index::GRIZZLY_BEARS;

fn parapet() -> CardIndex {
    card_index("4e73634a-9df8-4c36-b1d2-f366eae902fc")
}

/// `Parapet` ({1}{W}, Enchantment, `Coverage::Partial`) gives creatures you
/// control +0/+1, and only those.
#[test]
fn parapet_gives_your_creatures_plus_0_plus_1_and_not_the_opponents() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), GRIZZLY_BEARS])
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[parapet()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let theirs = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    assert_eq!(pt(&engine, mine), (2, 2));

    cast_from_hand(&mut engine, p0, parapet());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, parapet()).is_some());
    assert_eq!(pt(&engine, mine), (2, 3));
    assert_eq!(
        pt(&engine, theirs),
        (2, 2),
        "the opponent's creature gets nothing"
    );
}

/// Flash is left off because the sacrifice rider that goes with it is not
/// supported: the enchantment is castable in the main phase and not in the end
/// step.
#[test]
fn parapet_is_castable_only_at_sorcery_speed() {
    let p0 = PlayerId::new(0);
    let mut engine = lk::game(SEED, &[plains(), plains()], &[], &[parapet()], &[]);
    lk::float(
        &mut engine,
        p0,
        &[(ManaColor::Colorless, 1), (ManaColor::White, 1)],
    );
    assert!(
        lk::castable(&engine, p0, parapet()),
        "castable in the main phase with its mana floating"
    );

    lk::to_step(&mut engine, p0, Step::End);
    lk::float(
        &mut engine,
        p0,
        &[(ManaColor::Colorless, 1), (ManaColor::White, 1)],
    );
    assert!(
        !lk::castable(&engine, p0, parapet()),
        "not castable in the end step without flash"
    );
}
