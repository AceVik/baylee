//! `cards/instants/mv_4/cryptic_command.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// ---------------------------------------------------------------------------
// Schwarzrand: Cryptic Command ("choose two").
// ---------------------------------------------------------------------------

/// With no other spell on the stack "Counter target spell" has nothing to
/// point at, and a mode without a legal target cannot be chosen (CR
/// 700.2a); Cryptic Command cannot target itself. What is left is every
/// pair of the other three, each at {1}{U}{U}{U}: two modes exactly (CR
/// 700.2d), never one and never three. Tap and draw together: the
/// opponent's Elves and Gargoyle are tapped, seat 0's own Elves are not,
/// and seat 0 draws a card.
#[test]
fn cryptic_command_taps_their_creatures_and_draws() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[island(), island(), island(), island(), llanowar_elves()],
        )
        .battlefield(1, &[llanowar_elves(), darksteel_gargoyle(), island()])
        .hand(0, &[cryptic_command()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let mine = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    let theirs = [
        on_battlefield(&engine, p1, llanowar_elves()).unwrap(),
        on_battlefield(&engine, p1, darksteel_gargoyle()).unwrap(),
    ];
    let their_island = on_battlefield(&engine, p1, island()).unwrap();
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    let library = library_size(&engine, p0);
    let hand = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    cast_with_floating(&mut engine, p0, cryptic_command());
    let offered = choose_modes(&mut engine, p0, 0b1100);
    let cost = baylee_core::mana::ManaCost::parse("{1}{U}{U}{U}");
    assert_eq!(
        offered.iter().map(|o| (o.kind, o.cost)).collect::<Vec<_>>(),
        [
            (CastModeKind::Modes(0b0110), cost),
            (CastModeKind::Modes(0b1010), cost),
            (CastModeKind::Modes(0b1100), cost),
        ]
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(
        theirs.iter().all(|id| is_tapped(&engine, *id)),
        "theirs tap"
    );
    assert!(
        !is_tapped(&engine, mine),
        "its caster's own creature does not"
    );
    assert!(
        !is_tapped(&engine, their_island),
        "and a land is no creature"
    );
    assert_eq!(library_size(&engine, p0), library - 1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand,
        "Cryptic Command left the hand and one card came in"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert!(in_graveyard(&engine, p0, cryptic_command()).is_some());
}

/// With a Dark Ritual on the stack every pair of the four is offered, and
/// counter plus bounce takes two targets, the counter's first (CR 700.2c):
/// the Ritual is countered and adds no mana, the opponent's Elves go back
/// to their owner's hand, and nothing is tapped or drawn.
#[test]
fn cryptic_command_counters_a_spell_and_bounces_a_permanent() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[swamp(), island(), island(), island(), island()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[dark_ritual(), cryptic_command()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elves = on_battlefield(&engine, p1, llanowar_elves()).unwrap();
    tap_all_mana(&mut engine, p0);
    let library = library_size(&engine, p0);
    cast_with_floating(&mut engine, p0, dark_ritual());
    let ritual = on_stack(&engine, dark_ritual()).unwrap();
    cast_with_floating(&mut engine, p0, cryptic_command());
    let offered = choose_modes(&mut engine, p0, 0b0011);
    assert_eq!(
        offered.iter().map(|o| o.kind).collect::<Vec<_>>(),
        [0b0011, 0b0101, 0b0110, 0b1001, 0b1010, 0b1100].map(CastModeKind::Modes)
    );
    let _ = aim_at(&mut engine, p0, ritual);
    let _ = aim_at(&mut engine, p0, elves);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(ritual).map(|o| o.zone),
        Some(Zone::Graveyard)
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0, "countered");
    assert!(in_hand(&engine, p1, llanowar_elves()).is_some(), "bounced");
    assert_eq!(library_size(&engine, p0), library, "no card drawn");
}
