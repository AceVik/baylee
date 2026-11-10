//! `cards/creatures/mv_4/soltari_guerrillas.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn soltari_guerrillas() -> CardIndex {
    card_index("37da03c2-c03e-453c-8fbf-e1039faceb8c")
}

/// `Soltari Guerrillas` ({2}{R}{W}, 3/2 Creature — Soltari Soldier,
/// `Coverage::Partial`):
/// casts and enters as a 3/2 with shadow.
#[test]
fn soltari_guerrillas_casts_and_enters_with_shadow() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), mountain(), mountain()])
        .hand(0, &[soltari_guerrillas()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, soltari_guerrillas());
    pass_until(&mut engine, stack_is_empty);

    let soldier = on_battlefield(&engine, p0, soltari_guerrillas()).unwrap();
    let ch = engine.state().object(soldier).unwrap().characteristics();
    assert!(ch.keywords.contains(KeywordSet::SHADOW));
    assert_eq!((ch.power, ch.toughness), (Some(3), Some(2)));
}
