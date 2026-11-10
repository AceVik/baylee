//! `cards/creatures/mv_2/guardian_of_solitude.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Guardian of Solitude` prints a triggered ability under `Coverage::Implemented`:
/// "Whenever you cast a Spirit or Arcane spell, target creature gains flying until end of turn."
/// Because `Guardian of Solitude` is itself a Spirit, casting a second copy from hand triggers the ability,
/// allowing the caster to grant `KeywordSet::FLYING` to a grounded creature (`llanowar_elves`).
#[test]
fn guardian_of_solitude_grants_flying_on_spirit_cast() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1628, forest())
        .battlefield(
            0,
            &[guardian_of_solitude(), llanowar_elves(), island(), island()],
        )
        .hand(0, &[guardian_of_solitude()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("elf is seated");
    assert!(!keywords(&engine, elf).contains(KeywordSet::FLYING));

    cast_from_hand(&mut engine, p0, guardian_of_solitude());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice for Guardian of Solitude trigger");
    };
    assert!(options.contains(&elf), "Elf is a legal target creature");
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, elf).contains(KeywordSet::FLYING));
}
