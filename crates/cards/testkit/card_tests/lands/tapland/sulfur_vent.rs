//! `cards/lands/tapland/sulfur_vent.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sulfur Vent prints three lines: it enters tapped, it taps for {B}, and it
/// can be tapped and sacrificed for {U}{R}. All three are played on one board
/// holding nothing else — the land arrives by a real land drop (a permanent
/// seated through `starting_battlefield` never meets its enter modifier), is
/// offered neither {T} line while it lies tapped, and pays for the two lines
/// on the turns the untap step hands it back. The empty pool is what makes the
/// finish exact: one blue and one red and no black can only be the sacrifice
/// line, and the land has to be in its owner's graveyard for that line to have
/// been paid for at all.
#[test]
fn sulfur_vent_enters_tapped_then_taps_for_black_or_sacrifices_itself_for_blue_and_red() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest()).hand(0, &[sulfur_vent()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let vent = play_land(&mut engine, p0, sulfur_vent());
    assert!(
        entered_tapped(&engine, vent),
        "the printed enters-tapped line"
    );

    // A tapped land has no {T} to pay either line with, so neither is
    // offered. Both lines cost the tap symbol and no mana, so the empty pool
    // plays no part in their absence — the tap is the whole of it.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "a land drop leaves the seat holding priority, got {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == vent),
        "it entered tapped, so neither {{T}} line is payable this turn: {:?}",
        legal.abilities
    );

    // Its controller's next turn: the untap step stands it up (CR 502.3) and
    // only then is either {T} line payable.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, vent), "the untap step gave it back");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and nothing is floating on this board"
    );

    // Ability 0: "{T}: Add {B}." — a named colour, so it is mana in the pool
    // the moment it is activated and no colour is ever asked for.
    activate(&mut engine, p0, sulfur_vent(), 0);
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "`{{B}}` is fixed and not a choice, so nothing is asked on the way: {:?}",
        engine.pending()
    );
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1, "{{T}}: Add {{B}}");
    assert_eq!(pool.total(), 1, "one mana, and nothing beside it");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(
        is_tapped(&engine, vent),
        "the tap symbol was the whole price"
    );

    // Another cycle: the untap step again, and the {B} that was floating has
    // been emptied by the step boundaries in between (CR 500.5). That is what
    // makes the two mana below this line's own rather than last turn's.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, vent), "untapped again");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the pool is empty again"
    );

    // Ability 1: "{T}, Sacrifice this land: Add {U}{R}." Both halves of the
    // price are read after the fact — the land left the battlefield for its
    // owner's graveyard, and the pool holds exactly the two colours the card
    // names and none of the black the other line makes.
    activate(&mut engine, p0, sulfur_vent(), 1);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1, "the {{U}} half");
    assert_eq!(pool.available(ManaColor::Red), 1, "and the {{R}} half");
    assert_eq!(
        pool.available(ManaColor::Black),
        0,
        "no black: this is the other line, not the {{B}} one"
    );
    assert_eq!(pool.total(), 2, "two mana, and nothing else came with them");
    assert!(
        on_battlefield(&engine, p0, sulfur_vent()).is_none(),
        "\"Sacrifice this land\" is a cost, so the permanent is gone"
    );
    assert!(
        in_graveyard(&engine, p0, sulfur_vent()).is_some(),
        "and a sacrificed permanent goes to its owner's graveyard"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
}
