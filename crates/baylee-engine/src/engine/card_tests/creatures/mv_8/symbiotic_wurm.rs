//! `cards/creatures/mv_8/symbiotic_wurm.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

/// Symbiotic Wurm (`99e3b8ee-29e3-41de-af31-a4e8800c4619`) — {5}{G}{G}{G} Creature — Wurm 7/7:
/// "When this creature dies, create seven 1/1 green Insect creature tokens."
#[test]
fn symbiotic_wurm_creates_seven_insect_tokens_when_it_dies() {
    let wurm_card = card_index("99e3b8ee-29e3-41de-af31-a4e8800c4619");
    let p0 = PlayerId::new(0);

    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[wurm_card])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wurm = on_battlefield(&engine, p0, wurm_card).unwrap();
    kill(&mut engine, wurm);

    let tokens: Vec<_> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            let obj = engine.state().object(*id).unwrap();
            obj.controller == p0
                && obj.card.is_none()
                && obj
                    .characteristics()
                    .subtypes
                    .contains(baylee_core::generated::subtypes::creature::INSECT)
        })
        .collect();
    assert_eq!(tokens.len(), 7);
    for t in tokens {
        assert_eq!(pt(&engine, t), (1, 1));
    }
}
