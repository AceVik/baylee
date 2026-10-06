//! `cards/lands/pain/caldera_lake.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Caldera Lake prints "This land enters tapped" and then two lines that share
/// one `{T}`: "Add {C}", and "Add {U} or {R}. This land deals 1
/// damage to you." A single `{T}` can only pay for one of them, so the untap
/// step is part of the test rather than scenery — the land is *played* on turn
/// one and, being tapped, offers neither line at all; from the next turn it is
/// standing and the coloured line costs exactly one life, while the colourless
/// line on the turn after costs none. That puts the damage on the ability that
/// prints it and not on the land that carries it, and the empty pool at each
/// activation makes "exactly one blue" the whole of what the tap bought.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn caldera_lake_enters_tapped_and_pays_one_life_for_its_coloured_mana() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(4211, forest())
        .hand(0, &[caldera_lake()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its own first main phase"
    );

    // A real land drop, because `starting_battlefield` *places* a permanent
    // without an entry and would have left this one untapped (Cause::Setup,
    // no replacement effect looking at it).
    let lake = play_land(&mut engine, p0, caldera_lake());
    assert!(entered_tapped(&engine, lake), "Caldera Lake enters tapped");

    // Both printed lines want `{T}`, and `{T}` is spent for the turn, so the
    // turn it arrives is the turn it offers nothing at all.
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!(
            "a land drop leaves the seat holding priority, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "and it is the seat that played the land");
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == lake),
        "the only price either line asks is {{T}}, and a tapped land cannot \
         pay it: {:?}",
        legal.abilities
    );
    assert!(
        !legal.mana_abilities.contains(&lake),
        "no basic land type grants this one the CR 305.6 shortcut either"
    );

    // Across the opponent's turn and back: the untap step stands it up.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, lake),
        "the untap step stood the land back up"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for index in [0, 1] {
        assert!(
            legal.abilities.contains(&(lake, index)),
            "an untapped land offers both of its lines; ability {index} is \
             missing: {:?}",
            legal.abilities
        );
    }
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "no mana survives a phase boundary (CR 500.5), so the {{T}} is the \
         whole of the price and what is in the pool afterwards is what the \
         line made"
    );

    // Ability 1 is the coloured line. Two colours are producible, so it asks
    // which one — and the damage is on the same activation.
    let life_before = engine.state().players[0].life;
    activate(&mut engine, p0, caldera_lake(), 1);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "`Add {{U}} or {{R}}` is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat names the colour");
    assert_eq!(
        options,
        vec![ManaColor::Blue, ManaColor::Red],
        "the two colours the card prints, and colorless is no colour at all \
         (CR 105.4)"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the colours it offered");

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one mana off one tap, with nothing else in the pool"
    );
    assert_eq!(
        engine.state().players[0].life,
        life_before - 1,
        "\"This land deals 1 damage to you\": the life is the price of the \
         colour"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the damage lands on the land's controller, not across the table"
    );
    assert!(is_tapped(&engine, lake), "the {{T}} was paid");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to \
         resolve"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "and it asks nothing further on the way: got {:?}",
        engine.pending()
    );

    // The other line, on the turn after: the same tap, one colourless, no life.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, lake), "untapped again");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the blue made last turn went with the phase it was made in"
    );
    let life = engine.state().players[0].life;
    activate(&mut engine, p0, caldera_lake(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1,
        "{{T}}: Add {{C}}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one mana, and nothing coloured beside it"
    );
    assert_eq!(
        engine.state().players[0].life,
        life,
        "the colourless line prints no damage clause, so this activation costs \
         no life where the one before cost exactly one"
    );
    assert!(is_tapped(&engine, lake), "which tapped the land again");
    assert!(stack_is_empty(&engine), "no stack, no further question");
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again, got {:?}",
        engine.pending()
    );
}
