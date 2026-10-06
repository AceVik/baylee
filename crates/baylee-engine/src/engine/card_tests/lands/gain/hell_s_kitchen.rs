//! `cards/lands/gain/hell_s_kitchen.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hell's Kitchen prints three lines and one turn cycle reads all of them: it
/// enters tapped, its arrival gains its controller 1 life, and `{T}: Add {B}
/// or {R}`.
///
/// Each line is what keeps the other two honest. The board holds no other
/// permanent of p0's, so the life can only have come from playing *this*
/// card, and the land is read again on the following turn rather than in the
/// one it arrived in — a land that could pay in the turn it landed is a land
/// that never entered tapped, since CR 502.3 hands it back only in its
/// controller's own untap step.
///
/// The mana ability asks rather than choosing, so the pool afterwards names
/// the colour that was picked and nothing else, and both printed colours are
/// on the menu rather than one of them being assumed.
#[test]
fn hells_kitchen_enters_tapped_gains_a_life_and_taps_for_black_or_red() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1301, forest())
        .hand(0, &[hell_s_kitchen()])
        .life(0, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let life_before = engine.state().players[0].life;
    let land = play_land(&mut engine, p0, hell_s_kitchen());
    assert!(entered_tapped(&engine, land), "\"this land enters tapped\"");
    pass_until(&mut engine, |e| e.state().players[0].life > life_before);
    assert_eq!(
        engine.state().players[0].life,
        life_before + 1,
        "\"when this land enters, you gain 1 life\""
    );
    assert!(
        stack_is_empty(&engine),
        "the enters-trigger has finished resolving"
    );

    // The untap step of p0's next turn, which is where a land that arrived
    // tapped is handed back (CR 502.3) — the walk crosses p1's whole turn,
    // which is also the control for "nothing else on this board gains life".
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land), "the land untapped on schedule");

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    // `deeds` reads both offer lists, because a printed `{T}: Add …` may come
    // back either as the CR 305.6 shortcut or as an ordinary ability index.
    let offered = deeds(&legal, &[land]);
    assert_eq!(
        offered.len(),
        1,
        "an untapped land printing its own tap offers exactly that, and a \
         trigger is no activation: {offered:?}"
    );
    engine
        .apply(p0, offered[0].1.action(land))
        .expect("the offer is the ability it names");

    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{B}} or {{R}}\" is a choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat names the colour");
    assert_eq!(options.len(), 2, "two colours and no more: {options:?}");
    assert!(
        options.contains(&ManaColor::Black) && options.contains(&ManaColor::Red),
        "{{B}} and {{R}}: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the answers on the menu");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(pool.available(ManaColor::Red), 0, "and not both of them");
    assert_eq!(pool.total(), 1, "one land, one tap, one mana");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability never uses the stack"
    );
    assert!(is_tapped(&engine, land), "the land paid its own {{T}}");
}
