//! `cards/lands/utility/strip_mine.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The World Tree: "As long as you control six or more lands, lands you
/// control have '{T}: Add one mana of any color.'"
///
/// Five lands: nothing is granted. The sixth, a Strip Mine, played, turns
/// it on for every land its controller has, the Tree included, and a Forest
/// then taps for black. Six Mountains across the table count for nothing and
/// gain nothing. The Strip Mine sacrificed makes five again, and the grant
/// is gone.
#[test]
fn the_world_tree_colours_your_lands_from_the_sixth_on() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(212, forest())
        .battlefield(
            0,
            &[the_world_tree(), forest(), forest(), forest(), forest()],
        )
        .hand(0, &[strip_mine()])
        .battlefield(
            1,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let tree = on_battlefield(&engine, p0, the_world_tree()).expect("the Tree is out");
    let first = on_battlefield(&engine, p0, forest()).expect("a Forest is out");
    let theirs = on_battlefield(&engine, p1, mountain()).expect("a Mountain across the table");
    assert!(
        !offered_any_colour(&engine, first),
        "five lands: nothing is granted"
    );

    let sixth = play_land(&mut engine, p0, strip_mine());
    assert!(
        offered_any_colour(&engine, first),
        "six lands: the Forest has it"
    );
    assert!(
        offered_any_colour(&engine, sixth),
        "and the land that made six"
    );
    assert!(offered_any_colour(&engine, tree), "and the Tree itself");
    assert!(
        !offered_any_colour(&engine, theirs),
        "\"lands you control\": not the opponent's six"
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: first,
                ability_index: crate::choice::GRANTED_ABILITY,
            },
        )
        .expect("the granted mana ability activates");
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected a colour choice, got {:?}", engine.pending());
    };
    assert_eq!(options.len(), 5, "one mana of any color");
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black is a colour");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        1,
        "a Forest made black"
    );

    // Back to five: Strip Mine, the sixth land, is sacrificed to destroy
    // a Mountain across the table.
    activate(&mut engine, p0, strip_mine(), 1);
    aim_at(&mut engine, p0, theirs);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, strip_mine()).is_some());
    let untapped = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|id| {
            *id != first
                && engine.state().object(*id).is_some_and(|o| {
                    o.controller == p0
                        && o.card.is_some_and(|c| c.index == forest())
                        && !is_tapped(&engine, *id)
                })
        })
        .expect("an untapped Forest");
    assert!(
        !offered_any_colour(&engine, untapped),
        "five lands again: the grant is gone"
    );
    assert!(!offered_any_colour(&engine, tree));
}

/// Strip Mine prints two lines: `{T}: Add {C}` and `{T}, Sacrifice this land:
/// Destroy target land`. Both are played on one board, because each is the
/// other's control: the mana line costs nothing but its own tap and fills the
/// pool with exactly one colorless, while the destroy line costs the land
/// itself and no mana at all — so the `{C}` made by the copy that stays is
/// still floating when the copy that is sacrificed pays its price. The target
/// menu is the other half of the card: "target land" reaches across the table
/// (the opponent's Forest is on it) and is not "target permanent" (the
/// Llanowar Elves beside it is not). And because CR 601.2c names the target
/// before CR 601.2h pays, the Mine that is about to be sacrificed is still on
/// the battlefield, still untapped and still unreachable in any graveyard
/// while the question stands.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn strip_mine_sacrifices_itself_to_destroy_the_land_it_names() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[strip_mine(), strip_mine(), llanowar_elves()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let mines = all_on_battlefield(&engine, p0, strip_mine());
    assert_eq!(
        mines.len(),
        2,
        "two copies: one pays for the mana, one pays for the land"
    );
    let (mana_mine, destroy_mine) = (mines[0], mines[1]);
    let their_forest = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");

    // The card prints a mana ability, so both of its lines are ordinary
    // `(source, index)` entries in the offer: ability 0 is `{T}: Add {C}` and
    // ability 1 is the destroy. Neither price needs mana, so an empty pool
    // withholds nothing.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(mana_mine, 0)),
        "an untapped Strip Mine is a paid {{T}}, so its mana line is offered: {:?}",
        legal.abilities
    );
    assert!(
        legal.abilities.contains(&(destroy_mine, 1)),
        "and so is the line the card is played for: {:?}",
        legal.abilities
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: mana_mine,
                ability_index: 0,
            },
        )
        .expect("the mana line costs its own tap and nothing else");
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "`{{C}}` is fixed, so the mana line asks nothing on the way (CR 605.1), got {:?}",
        engine.pending()
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the mana is already here"
    );
    assert!(is_tapped(&engine, mana_mine), "the tap was the whole price");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1,
        "{{T}}: Add {{C}}"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        0,
        "and the Elf beside it was never tapped for anything: the whole pool \
         is the one colorless the Mine made"
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: destroy_mine,
                ability_index: 1,
            },
        )
        .expect("the destroy line costs the land itself and no mana");
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target land\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert_eq!((min, max), (1, 1), "one land, and the ability asks once");
    assert!(
        options.contains(&their_forest),
        "\"target land\" names no side of the table, so the Forest across it \
         is on the menu: {options:?}"
    );
    assert!(
        !options.contains(&my_elf),
        "the Elf is a permanent and no land: {options:?}"
    );
    // CR 601.2c names the target first and CR 601.2h pays afterwards, so the
    // Mine is still standing and the mana still floating while this is open.
    assert!(
        !is_tapped(&engine, destroy_mine),
        "the {{T}} is the last step of the activation, not the first"
    );
    assert!(
        in_graveyard(&engine, p0, strip_mine()).is_none(),
        "and the sacrifice has not happened yet either"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the destroy line charges no mana, so the {{C}} is untouched"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_forest],
            },
        )
        .expect("the Forest across the table was one of the options it enumerated");
    assert!(
        in_graveyard(&engine, p0, strip_mine()).is_some(),
        "\"Sacrifice this land\" is part of the price and CR 601.2h pays it last"
    );
    assert!(
        !stack_is_empty(&engine),
        "destroying a land is no mana ability, so the ability is waiting on the stack"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, forest()).is_some(),
        "\"Destroy target land\": the Forest the ability named is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_none(),
        "and has left the battlefield, which is the destruction itself"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the Elf the filter declined never moved"
    );
    assert!(
        on_battlefield(&engine, p0, strip_mine()).is_some(),
        "and only the copy that paid was sacrificed: the other is still standing"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the whole price was a tap and the land, so the {{C}} the first copy \
         made is still in the pool"
    );
}
