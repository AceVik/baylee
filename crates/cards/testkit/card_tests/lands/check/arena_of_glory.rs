//! `cards/lands/check/arena_of_glory.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Arena of Glory: "This land enters tapped unless you control a Mountain." / "{T}: Add {R}." / "{R}, {T}, Exert this land: Add {R}{R}..."
/// Controlling a Mountain allows Arena of Glory to enter untapped and immediately tap for red mana.
#[test]
fn arena_of_glory_enters_untapped_with_mountain_and_taps_for_red() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(303, forest())
        .battlefield(0, &[mountain()])
        .hand(0, &[arena_of_glory()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, arena_of_glory());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, arena_of_glory(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1);
    assert!(is_tapped(&engine, land));
}

/// The condition names your Mountain subtype, not merely a basic land.
#[test]
fn arena_of_glory_entry_requires_your_mountain_including_nonbasic() {
    for (own_lands, tapped) in [
        (vec![], true),
        (vec![forest()], true),
        (vec![baylee_core::generated::index::TAIGA], false),
    ] {
        let p0 = PlayerId::new(0);
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &own_lands)
            .battlefield(1, &[mountain()])
            .hand(0, &[arena_of_glory()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);
        let arena = play_land(&mut engine, p0, arena_of_glory());
        assert_eq!(entered_tapped(&engine, arena), tapped);
    }
}

/// Each red mana can grant haste to a different spell; exert skips just the next untap.
#[test]
fn batch55_arena_of_glory_splits_haste_mana_and_skips_one_untap() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let goblin = baylee_core::generated::index::MONS_S_GOBLIN_RAIDERS;
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[arena_of_glory(), mountain()])
        .hand(0, &[goblin, goblin])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let arena = on_battlefield(&engine, p0, arena_of_glory()).unwrap();
    tap_mana_except(&mut engine, p0, arena);
    activate(&mut engine, p0, arena_of_glory(), 1);
    assert!(
        stack_is_empty(&engine),
        "the exert ability is a mana ability"
    );
    assert!(is_tapped(&engine, arena));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        2
    );
    cast_with_floating(&mut engine, p0, goblin);
    pass_until(&mut engine, stack_is_empty);
    cast_with_floating(&mut engine, p0, goblin);
    pass_until(&mut engine, stack_is_empty);
    let goblins: Vec<_> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|card| card.index == goblin))
        })
        .collect();
    assert_eq!(goblins.len(), 2);
    for id in &goblins {
        assert!(keywords(&engine, *id).contains(KeywordSet::HASTE));
    }
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending() else {
        unreachable!()
    };
    assert!(
        goblins.iter().all(|id| attackers.contains(id)),
        "both newly cast creatures can attack"
    );
    reach_their_main_phase(&mut engine, p1);
    for id in &goblins {
        assert!(!keywords(&engine, *id).contains(KeywordSet::HASTE));
    }
    reach_their_main_phase(&mut engine, p0);
    assert!(is_tapped(&engine, arena), "next untap is skipped");
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, arena), "following untap is normal");
}

/// Exert mana is unrestricted, but noncreature spells do not give their tokens haste.
#[test]
fn batch55_arena_of_glory_noncreature_token_spell_does_not_grant_haste() {
    let p0 = PlayerId::new(0);
    let fodder = baylee_core::generated::index::DRAGON_FODDER;
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[arena_of_glory(), mountain()])
        .hand(0, &[fodder])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let arena = on_battlefield(&engine, p0, arena_of_glory()).unwrap();
    tap_mana_except(&mut engine, p0, arena);
    activate(&mut engine, p0, arena_of_glory(), 1);
    cast_with_floating(&mut engine, p0, fodder);
    pass_until(&mut engine, stack_is_empty);
    let goblins = tokens_of(&engine, p0);
    assert_eq!(goblins.len(), 2);
    for id in &goblins {
        assert_eq!(pt(&engine, *id), (1, 1));
        assert!(!keywords(&engine, *id).contains(KeywordSet::HASTE));
    }
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending() else {
        unreachable!()
    };
    assert!(goblins.iter().all(|id| !attackers.contains(id)));
}

/// An Arena borrowed with Conquer and returned after exert untaps for its original controller.
#[test]
fn batch55_arena_of_glory_borrowed_exert_does_not_skip_original_controllers_untap() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let conquer = baylee_core::generated::index::CONQUER;
    let mut engine = Duel::new(SEED, forest())
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
        .battlefield(1, &[arena_of_glory(), forest(), forest()])
        .hand(0, &[conquer])
        .hand(1, &[naturalize()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    // The opponent's Arena entered tapped without a Mountain. Let its own
    // untap happen before borrowing it; stealing a tapped land does not untap it.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    let arena = on_battlefield(&engine, p1, arena_of_glory()).unwrap();
    assert!(
        !is_tapped(&engine, arena),
        "the borrowed land starts untapped"
    );
    cast_from_hand(&mut engine, p0, conquer);
    aim_at(&mut engine, p0, arena);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(arena).unwrap().controller, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1
    );
    activate(&mut engine, p0, arena_of_glory(), 1);
    assert!(is_tapped(&engine, arena));
    let aura = on_battlefield(&engine, p0, conquer).unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    cast_from_hand(&mut engine, p1, naturalize());
    aim_at(&mut engine, p1, aura);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, conquer).is_some());
    assert_eq!(engine.state().object(arena).unwrap().controller, p1);
    assert!(is_tapped(&engine, arena));
    reach_their_main_phase(&mut engine, p1);
    assert!(
        !is_tapped(&engine, arena),
        "p1 did not exert the land; their untap is normal"
    );
}
