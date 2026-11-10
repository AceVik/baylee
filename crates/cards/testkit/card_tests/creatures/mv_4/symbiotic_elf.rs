//! `cards/creatures/mv_4/symbiotic_elf.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

/// Symbiotic Elf (`cfaa0d5d-d9f1-4586-a7b1-6576a58c0f1e`) — {3}{G} Creature — Elf 2/2:
/// "When this creature dies, create two 1/1 green Insect creature tokens."
#[test]
fn symbiotic_elf_creates_two_insect_tokens_when_it_dies() {
    let elf_card = card_index("cfaa0d5d-d9f1-4586-a7b1-6576a58c0f1e");
    let p0 = PlayerId::new(0);

    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[elf_card])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, elf_card).unwrap();
    kill(&mut engine, elf);

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
    assert_eq!(tokens.len(), 2);
    for t in tokens {
        assert_eq!(pt(&engine, t), (1, 1));
    }
}
