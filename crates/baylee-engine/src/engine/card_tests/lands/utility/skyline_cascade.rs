//! `cards/lands/utility/skyline_cascade.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Skyline Cascade is ``Coverage::Partial``: "This land enters tapped" and
/// "{T}: Add {U}" are written, while "when this land enters, target creature
/// an opponent controls doesn't untap during its controller's next untap
/// step" is not — no DSL variant expresses a duration-bound does-not-untap.
/// The land is therefore **played** (`PlayLand`, not a `starting_battlefield`
/// placement, which would arrive untapped whatever the card says) so the
/// printed enter modifier has to do the tapping, and the missing trigger is
/// read where it would have to speak: the game hands priority straight back
/// instead of standing on a target question, while the Elf across the table
/// is the creature that question would have named. One turn later the untap
/// step frees the land and the printed mana ability pays, which is the other
/// half of the card.
#[test]
fn skyline_cascade_enters_tapped_taps_for_blue_and_asks_nothing_of_the_creature_across_the_table() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(41, forest())
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[skyline_cascade()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");

    let land = play_land(&mut engine, p0, skyline_cascade());
    assert!(
        entered_tapped(&engine, land),
        "the printed `EnterModifier::Tapped` is a real entry this way — a \
         `starting_battlefield` placement would have arrived untapped"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the land entered and the game went straight back to priority: the \
         does-not-untap trigger is the `Coverage::Partial` gap, so no target \
         was ever asked ({:?})",
        engine.pending()
    );
    assert!(
        !is_tapped(&engine, theirs),
        "and the creature the missing trigger would have aimed at is \
         untouched by anything"
    );

    // The untap step, a turn later: nothing keeps the Cascade down, and the
    // printed `{T}: Add {U}` is the only mana route on this board.
    reach_their_main_phase(&mut engine, p1);
    assert!(walk_to_own_main(&mut engine, p0), "p0 takes another turn");
    assert!(
        !is_tapped(&engine, land),
        "CR 502.3 ran and the untap step freed the land it had put down"
    );

    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(
        taken, 1,
        "the Cascade's own {{T}}: Add {{U}} is the whole of what can be tapped"
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "one blue, and the colour the card prints"
    );
    assert_eq!(
        pool.total(),
        1,
        "off the one tap, with nothing else floated"
    );
    assert!(is_tapped(&engine, land), "which tapped the land for it");
}
