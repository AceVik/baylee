//! `cards/lands/cycling/lonely_sandbar.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lonely Sandbar prints three lines and one turn of it reads all three: it
/// enters tapped, it has `{T}: Add {U}`, and it has `Cycling {U}`, which is
/// `{U}, Discard this card: Draw a card.` One copy is cycled out of the hand
/// off an Island — the card lands in the graveyard, the library is one
/// shorter and the {U} is gone from the pool — and the second is played as
/// the turn's land drop, where it arrives tapped and so can produce nothing
/// until its controller's next untap step. That last step is why the second
/// copy is played at all: reading `{U}` off it in the turn it entered would
/// pass just as well on a land that never came in tapped.
#[test]
fn lonely_sandbar_cycles_a_card_and_taps_for_blue_only_after_its_untap_step() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island()])
        .hand(0, &[lonely_sandbar(), lonely_sandbar()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let library_before = library_size(&engine, p0);
    // `Cycling {U}` is a mana cost and `can_afford` reads the pool, not the
    // untapped lands: the Island is tapped before anything is claimed about
    // the ability.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1,
        "one Island is the whole board, so one blue"
    );

    // Ability 0 is the printed mana ability, 1 is the cycling line.
    activate(&mut engine, p0, lonely_sandbar(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, lonely_sandbar()).is_some(),
        "\"Discard this card\" is a cost, so the cycler is already in the \
         graveyard before the ability resolves"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "and the draw is the effect: exactly one card off the top"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{U}} was the other half of the cost"
    );

    // The second copy is the land drop, and it comes in tapped.
    let sandbar = play_land(&mut engine, p0, lonely_sandbar());
    assert!(
        entered_tapped(&engine, sandbar),
        "\"This land enters tapped\""
    );

    // A tapped land has no {T} to offer, so the mana line is read on the far
    // side of a turn cycle — which is also where the untap step stands it
    // back up.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, sandbar),
        "the untap step came round and stood the Sandbar back up"
    );

    activate(&mut engine, p0, lonely_sandbar(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "{{T}}: Add {{U}} — the printed ability, and the only mana this board \
         made that was not spent"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(is_tapped(&engine, sandbar), "the land paid its own {{T}}");
}
