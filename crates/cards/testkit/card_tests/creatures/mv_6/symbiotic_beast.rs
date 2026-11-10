//! `cards/creatures/mv_6/symbiotic_beast.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

/// Symbiotic Beast (`70ecc25b-2004-4a3c-80c3-6d97bf0711eb`) — {4}{G}{G} Creature — Insect Beast 4/4:
/// "When this creature dies, create four 1/1 green Insect creature tokens."
#[test]
fn symbiotic_beast_creates_four_insect_tokens_when_it_dies() {
    let beast_card = card_index("70ecc25b-2004-4a3c-80c3-6d97bf0711eb");
    let p0 = PlayerId::new(0);

    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[beast_card])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let beast = on_battlefield(&engine, p0, beast_card).unwrap();
    kill(&mut engine, beast);

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
    assert_eq!(tokens.len(), 4);
    for t in tokens {
        assert_eq!(pt(&engine, t), (1, 1));
    }
}
