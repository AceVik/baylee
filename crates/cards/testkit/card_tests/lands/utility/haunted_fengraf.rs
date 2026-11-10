//! `cards/lands/utility/haunted_fengraf.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn haunted_fengraf_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(112, forest())
        .battlefield(0, &[haunted_fengraf()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, haunted_fengraf()).expect("Haunted Fengraf deployed");
    activate(&mut engine, p0, haunted_fengraf(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

#[test]
fn fengraf_returns_one_random_creature_without_prompting_and_replays_identically() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut found = std::collections::BTreeSet::new();
    for seed in 1..17 {
        let mut hashes = Vec::new();
        for _ in 0..2 {
            let mut e = Duel::new(seed, forest())
                .battlefield(
                    0,
                    &[
                        index::HAUNTED_FENGRAF,
                        index::GRIZZLY_BEARS,
                        index::LLANOWAR_ELVES,
                        index::MIND_STONE,
                    ],
                )
                .battlefield(1, &[index::SHIVAN_DRAGON])
                .start();
            keep_mulligans(&mut e);
            reach_main_phase(&mut e, p0);
            let bear = on_battlefield(&e, p0, index::GRIZZLY_BEARS).unwrap();
            let elf = on_battlefield(&e, p0, index::LLANOWAR_ELVES).unwrap();
            let stone = on_battlefield(&e, p0, index::MIND_STONE).unwrap();
            let theirs = on_battlefield(&e, p1, index::SHIVAN_DRAGON).unwrap();
            bury(&mut e, &[bear, elf, stone, theirs]);
            e.dev_state_mut(p0).unwrap().players[0]
                .mana_pool
                .add(ManaColor::Colorless, 3);
            e.refresh_offer();
            activate(&mut e, p0, index::HAUNTED_FENGRAF, 1);
            pass_until(&mut e, stack_is_empty);
            let picked = [bear, elf]
                .into_iter()
                .filter(|id| e.state().object(*id).unwrap().zone == Zone::Hand)
                .collect::<Vec<_>>();
            assert_eq!(picked.len(), 1);
            found.insert(e.state().object(picked[0]).unwrap().card.unwrap().index);
            assert_eq!(e.state().object(stone).unwrap().zone, Zone::Graveyard);
            assert_eq!(e.state().object(theirs).unwrap().zone, Zone::Graveyard);
            assert!(in_graveyard(&e, p0, index::HAUNTED_FENGRAF).is_some());
            hashes.push(e.snapshot_hash());
        }
        assert_eq!(hashes[0], hashes[1]);
    }
    assert_eq!(
        found.len(),
        2,
        "neither eligible card is selected unconditionally"
    );
}
