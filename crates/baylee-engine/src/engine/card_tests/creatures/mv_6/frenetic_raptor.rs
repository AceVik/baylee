//! `cards/creatures/mv_6/frenetic_raptor.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Frenetic Raptor (`4b875669-f5c4-4f52-95be-ac52b0e33ce1`) — {5}{R} Creature — Dinosaur Beast 6/6:
/// "Beasts can't block."
#[test]
fn frenetic_raptor_prevents_all_beasts_from_blocking() {
    let raptor_card = card_index("4b875669-f5c4-4f52-95be-ac52b0e33ce1");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .battlefield(1, &[index::RAVENOUS_BALOTH, grizzly_bears()])
        .hand(0, &[raptor_card])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana_but(&mut engine, p0, None);
    cast_with_floating(&mut engine, p0, raptor_card);
    pass_until(&mut engine, stack_is_empty);

    let raptor = on_battlefield(&engine, p0, raptor_card).unwrap();
    let opp_beast = on_battlefield(&engine, p1, index::RAVENOUS_BALOTH).unwrap();
    let opp_bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    assert!(keywords(&engine, raptor).contains(KeywordSet::CANT_BLOCK));
    assert!(keywords(&engine, opp_beast).contains(KeywordSet::CANT_BLOCK));
    assert!(!keywords(&engine, opp_bear).contains(KeywordSet::CANT_BLOCK));
}
