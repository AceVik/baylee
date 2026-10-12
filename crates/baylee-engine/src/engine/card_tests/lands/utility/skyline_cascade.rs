//! `cards/lands/utility/skyline_cascade.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Real entry asks for the opposing creature, leaves it untapped, and the
/// Cascade itself untaps normally to produce blue on its next turn.
#[test]
fn skyline_cascade_enters_tapped_targets_opposing_creature_and_later_taps_for_blue() {
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
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("Cascade must target an opposing creature");
    };
    assert_eq!(options, &[theirs]);
    aim_at(&mut engine, p0, theirs);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !is_tapped(&engine, theirs),
        "suppression does not tap the creature"
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
