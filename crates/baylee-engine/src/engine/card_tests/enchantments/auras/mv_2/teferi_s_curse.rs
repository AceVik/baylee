//! `cards/enchantments/auras/mv_2/teferi_s_curse.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn teferi_s_curse() -> CardIndex {
    card_index("6e422d5e-d9f5-492d-b82a-31f7274d4153")
}

/// Teferi's Curse (`{1}{U}`, `Coverage::Implemented`): Aura enchanting an
/// artifact or creature that grants phasing to enchanted permanent.
#[test]
fn teferi_s_curse_grants_phasing_to_enchanted_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), grizzly_bears()])
        .hand(0, &[teferi_s_curse()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    cast_from_hand(&mut engine, p0, teferi_s_curse());
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, bear).contains(KeywordSet::PHASING));

    // On p0's next untap step, Grizzly Bears phases out.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        engine
            .state()
            .object(bear)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );
}
