//! `cards/lands/cycling/smoldering_crater.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Smoldering Crater prints three lines and one main phase plays all of them:
/// it enters tapped, it taps for {R}, and it cycles for {2} — "Discard this
/// card: Draw a card", which is an ability the card activates *from hand* and
/// not a spell it casts.
///
/// Each line needs a different witness. The Crater standing on the battlefield
/// is the only red source on a board of Forests, so one red in the pool can
/// only have come off its printed `{T}: Add {R}`; the {2} the cycling charges
/// comes out of that same pool, so the count afterwards says the cost was
/// really paid and not waived; and the Crater played afterwards is read off a
/// real `PlayLand` — a permanent seeded by `starting_battlefield` is placed
/// without its entry, so a land that "enters tapped" is only ever tapped when
/// it is played.
#[test]
fn smoldering_crater_cycles_from_hand_for_two_and_enters_tapped_when_played() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), smoldering_crater()])
        // Two copies, because the first is spent: cycling *discards* it, and
        // the land drop below needs a card that is still in hand.
        .hand(0, &[smoldering_crater(), smoldering_crater()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Mana first: whether an ability is offered is read off the pool, not off
    // the untapped lands beside it.
    tap_all_mana(&mut engine, p0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 3, "two Forests and the Crater, one mana each");
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "the Crater's printed {{T}}: Add {{R}} — no Forest on this board is red"
    );

    // Cycling {2} is the card's second ability — after the mana ability the
    // card prints — and it is offered while the card sits in hand.
    let crater_in_hand = in_hand(&engine, p0, smoldering_crater()).expect("a Crater is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(crater_in_hand, 1)),
        "Cycling {{2}} is activated from hand: {:?}",
        legal.abilities
    );

    let library_before = library_size(&engine, p0);
    activate(&mut engine, p0, smoldering_crater(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{2}} came out of the pool, and the red is what is left of it"
    );
    assert!(
        in_graveyard(&engine, p0, smoldering_crater()).is_some(),
        "the card itself is the other half of the cost: cycling discards it"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "and the ability draws a card — cycling is not merely a discard"
    );

    // The second Crater is played as the land drop, through a real PlayLand,
    // which is the only path that runs an enters-tapped modifier.
    let land = play_land(&mut engine, p0, smoldering_crater());
    assert!(
        is_tapped(&engine, land),
        "This land enters tapped — a seeded permanent skips its entry, so this \
         is the card's first line and not the harness'"
    );
}
