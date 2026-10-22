//! `cards/enchantments/mv_3/katabatic_winds.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn katabatic_winds() -> CardIndex {
    card_index("70446c56-07de-4d7f-94ae-4da88fba41cf")
}

/// `Katabatic Winds` ({2}{G}, Enchantment, `Coverage::Partial`) has phasing and
/// prevents creatures with flying from attacking or blocking.
#[test]
fn katabatic_winds_prevents_flying_creatures_from_attacking_or_blocking() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), serra_angel()])
        .hand(0, &[katabatic_winds()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, katabatic_winds());
    pass_until(&mut engine, stack_is_empty);
    let winds = on_battlefield(&engine, p0, katabatic_winds()).unwrap();
    assert!(keywords(&engine, winds).contains(KeywordSet::PHASING));

    let angel = on_battlefield(&engine, p0, serra_angel()).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(angel, Defender::Player(p1))],
                },
            )
            .is_err(),
        "creature with flying cannot attack while Katabatic Winds is on the battlefield"
    );
}
