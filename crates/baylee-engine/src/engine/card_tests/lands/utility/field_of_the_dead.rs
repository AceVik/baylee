//! `cards/lands/utility/field_of_the_dead.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Field of the Dead prints `This land enters tapped.`, `{{T}}: Add {{C}}.`, and `Whenever this land or another land you control enters, if you control seven or more lands with different names, create a 2/2 black Zombie creature token.`
///
/// Played as the seventh land with a distinct name alongside `forest()`,
/// `plains()`, `island()`, `swamp()`, `mountain()`, and `badlands()`, it
/// enters tapped, and its own entry triggers: seven names, one Zombie. On the
/// following turn, it untaps and ability 0 produces one colorless mana.
/// (The trigger was off the card while no condition could count names.)
#[test]
fn field_of_the_dead_enters_tapped_makes_a_zombie_as_the_seventh_name_and_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                plains(),
                island(),
                swamp(),
                mountain(),
                badlands(),
            ],
        )
        .hand(0, &[field_of_the_dead()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, field_of_the_dead());
    assert!(
        entered_tapped(&engine, land),
        "field of the dead enters tapped"
    );

    assert!(
        !stack_is_empty(&engine),
        "\"whenever this land … enters\": its own entry triggers"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        tokens_of(&engine, p0).len(),
        1,
        "seven lands, seven names: one 2/2 Zombie"
    );

    // Untap on the next turn to activate the mana ability.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    assert!(!is_tapped(&engine, land), "untaps on next turn");

    activate(&mut engine, p0, field_of_the_dead(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert_eq!(pool.total(), 1);
    assert!(is_tapped(&engine, land));
}

/// The intervening-if counts *names*, not lands: seven lands where two are
/// Forests are six names, and Field of the Dead's entry makes nothing. The
/// board differs from the test above by one Badlands turned into a second
/// Forest, so the land count is the same and only the names moved.
#[test]
fn field_of_the_dead_makes_nothing_while_two_lands_share_a_name() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), plains(), island(), swamp(), mountain(), forest()],
        )
        .hand(0, &[field_of_the_dead()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    play_land(&mut engine, p0, field_of_the_dead());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(lands_of(&engine, p0).len(), 7);
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "seven lands but six names: no Zombie"
    );
}
