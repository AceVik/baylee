//! `cards/lands/manlands/forbidding_watchtower.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Forbidding Watchtower enters tapped, taps for {W}, and for {1}{W} becomes
/// a 1/5 white Soldier that is **still a land**.
///
/// The two halves want two different turns, and that is what the scenario is
/// built around: on the turn it is played the land is tapped, so its own {T}
/// is not even offered — the only reading that tells the printed
/// `EnterModifier::Tapped` from a land that merely had nothing to do. The
/// turn after, the same board offers it, and the white mana in the pool has
/// no other source because the Plains beside it is named as the one kept
/// back.
///
/// `AddType` rather than `SetType` is what the last assertions are for: a
/// 1/5 Soldier that had stopped being a land would read exactly the same in
/// the card file and lose the whole card.
#[test]
fn forbidding_watchtower_enters_tapped_taps_for_white_and_turns_into_a_1_5_soldier_land() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains()])
        .hand(0, &[forbidding_watchtower()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let tower = play_land(&mut engine, p0, forbidding_watchtower());
    assert!(
        entered_tapped(&engine, tower),
        "\"This land enters tapped\""
    );
    assert!(
        !types(&engine, tower).contains(TypeSet::CREATURE),
        "and it is a land and nothing else until something animates it"
    );

    // Tapped, so its own {T} cannot be paid; the offer is the readable half
    // of the printed entry.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("a land drop uses no stack, got {:?}", engine.pending())
    };
    assert!(
        !legal.mana_abilities.contains(&tower)
            && !legal.abilities.iter().any(|(source, _)| *source == tower),
        "a tapped land has no {{T}} to spend, so its mana ability is in \
         neither list: {:?} / {:?}",
        legal.mana_abilities,
        legal.abilities
    );

    // A whole turn cycle, so the untap step hands it back.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, tower), "the untap step ran");

    // The Watchtower alone: the Plains is the one source kept back, so the
    // white mana in the pool afterwards has no other source on this board.
    tap_all_mana_but(&mut engine, p0, Some(plains()));
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1, "{{T}}: Add {{W}}");
    assert_eq!(pool.total(), 1, "and nothing else is floating");
    assert!(is_tapped(&engine, tower), "which is what paid for it");

    // The other half of {1}{W}. The Watchtower is already tapped for its own
    // mana and the animation asks for no untapped source.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "{{1}}{{W}} is exactly two, and one of them came off the land itself"
    );

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(tower, 1)),
        "with the mana floating the animation is affordable and offered: {:?}",
        legal.abilities
    );

    // Ability 0 is the mana ability; 1 is "{1}{W}: This land becomes a 1/5
    // white Soldier creature until end of turn. It's still a land."
    activate(&mut engine, p0, forbidding_watchtower(), 1);
    assert!(
        !stack_is_empty(&engine),
        "the animation is no mana ability, so it uses the stack"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{1}}{{W}} was paid out of the pool before it resolves"
    );
    pass_until(&mut engine, stack_is_empty);

    let kinds = types(&engine, tower);
    assert!(
        kinds.contains(TypeSet::CREATURE),
        "\"This land becomes a 1/5 white Soldier creature\": {kinds:?}"
    );
    assert!(
        kinds.contains(TypeSet::LAND),
        "\"It's still a land\" — a `SetType` that dropped the land type would \
         read identically in the card file: {kinds:?}"
    );
    assert_eq!(pt(&engine, tower), (1, 5), "and the body the card prints");
    assert!(
        on_battlefield(&engine, p0, plains()).is_some(),
        "the Plains kept back never moved"
    );
}
