//! `cards/lands/cycling/remote_isle.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Remote Isle prints three lines and this board makes each of them a *move*
/// rather than a comparison against the card file: it enters tapped, it taps
/// for {U}, and it cycles for {2}. Two copies are needed because one card
/// cannot show both halves in the same turn — the copy played as the land
/// drop is what proves the enter modifier ran (and that a land which came in
/// tapped has no `{T}` to offer that turn), while the copy left in hand is
/// the one that gets cycled, which leaves the played copy standing to untap
/// on the following turn and to be the only untapped source the single blue
/// mana could have come from.
#[test]
fn remote_isle_enters_tapped_taps_for_blue_and_cycles_its_other_copy_away() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[remote_isle(), remote_isle()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The land drop, played for real: the enter modifier is the only thing
    // that can have tapped it.
    let isle = play_land(&mut engine, p0, remote_isle());
    assert!(
        entered_tapped(&engine, isle),
        "\"This land enters tapped.\""
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(source, _)| *source == isle),
        "and a land that came in tapped has no {{T}} to offer this turn: {:?}",
        legal.abilities
    );

    // {2} for the cycling, off the three Islands — tapped before the claim,
    // because the offer reads the pool and not the untapped lands.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        3,
        "three Islands, three blue"
    );
    let library_before = library_size(&engine, p0);

    // Ability 1 is the printed cycling; ability 0 is the mana ability, and it
    // belongs to the copy lying tapped on the battlefield.
    activate(&mut engine, p0, remote_isle(), 1);
    assert!(!stack_is_empty(&engine), "cycling is no mana ability");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, remote_isle()).is_some(),
        "the discard is part of the cycling cost, so the card is in its owner's graveyard"
    );
    assert!(
        in_hand(&engine, p0, remote_isle()).is_none(),
        "and it left the hand to get there"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"Draw a card\" — one card off the top"
    );

    // A turn around, so the land the enter modifier tapped is standing again.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, isle),
        "its controller's untap step stands it back up"
    );

    activate(&mut engine, p0, remote_isle(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "\"{{T}}: Add {{U}}\" — one blue, and the Remote Isle is the only \
         source on this board that made any"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
}
