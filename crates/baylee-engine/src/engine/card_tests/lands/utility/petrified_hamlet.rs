//! `cards/lands/utility/petrified_hamlet.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Petrified Hamlet prints `When this land enters, choose a land card name.`, `Activated abilities of sources with the chosen name can't be activated unless they're mana abilities.`, `Lands with the chosen name have "{{T}}: Add {{C}}."`, and `{{T}}: Add {{C}}.`
///
/// Under `Coverage::Partial`, the name-choosing ETB and static ability lock are omitted because naming card choices and name-based activation locks are unsupported.
/// Playing Petrified Hamlet from hand enters the battlefield untapped and leaves the player at quiet priority without prompting for a card name.
/// Activating ability 0 produces one colorless mana in `pool.available(ManaColor::Colorless)` and leaves the land tapped.
#[test]
fn petrified_hamlet_enters_untapped_and_taps_for_colorless() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[petrified_hamlet()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hamlet = play_land(&mut engine, p0, petrified_hamlet());
    assert!(
        !entered_tapped(&engine, hamlet),
        "petrified hamlet enters untapped"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "no ETB name choice prompt is asked under `Coverage::Partial`"
    );

    activate(&mut engine, p0, petrified_hamlet(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert_eq!(pool.total(), 1);
    assert!(is_tapped(&engine, hamlet));
}

/// A duel in which player 0 has just played Petrified Hamlet and named `name`,
/// and player 1 (whose board is `their_board`) holds priority.
#[track_caller]
fn hamlet_naming(their_board: &[CardIndex], name: CardIndex) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .hand(0, &[petrified_hamlet()])
        .battlefield(1, their_board)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    play_land(&mut e, p0, petrified_hamlet());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCardName { .. })
    });
    e.apply(
        p0,
        PlayerAction::ChooseCardName {
            card: name,
            face: 0,
        },
    )
    .expect("a land card name");
    pass_until(&mut e, stack_is_empty);
    e.apply(p0, PlayerAction::PassPriority).unwrap();
    assert!(matches!(e.pending(), Pending::Priority { player, .. } if *player == PlayerId::new(1)));
    e
}

/// "Lands with the chosen name have "{T}: Add {C}."" A land with no mana
/// ability of its own, Maze of Ith, is given one: its controller taps it for
/// {C}.
#[test]
fn petrified_hamlet_gives_the_named_land_a_colorless_mana_ability() {
    use baylee_core::generated::index;
    let p1 = PlayerId::new(1);
    let mut e = hamlet_naming(&[index::MAZE_OF_ITH], index::MAZE_OF_ITH);
    let maze = on_battlefield(&e, p1, index::MAZE_OF_ITH).unwrap();
    let Pending::Priority { legal, .. } = e.pending().clone() else {
        unreachable!()
    };
    assert!(
        legal
            .abilities
            .contains(&(maze, crate::choice::GRANTED_ABILITY)),
        "the granted {{T}}: Add {{C}} is on offer: {:?}",
        legal.abilities
    );
    e.apply(
        p1,
        PlayerAction::ActivateAbility {
            source: maze,
            ability_index: crate::choice::GRANTED_ABILITY,
        },
    )
    .expect("the granted mana ability activates");
    let pool = &e.state().players[1].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert_eq!(pool.total(), 1);
    assert!(is_tapped(&e, maze), "and it paid its own {{T}}");
}

/// "Activated abilities of sources with the chosen name can't be activated
/// unless they're mana abilities." The lock is the name's: a Wasteland named
/// is locked, a Strip Mine beside it is not.
#[test]
fn petrified_hamlet_locks_the_named_land_and_not_other_lands() {
    use baylee_core::generated::index;
    let p1 = PlayerId::new(1);
    let e = hamlet_naming(&[index::WASTELAND, index::STRIP_MINE], index::WASTELAND);
    let wasteland = on_battlefield(&e, p1, index::WASTELAND).unwrap();
    let strip_mine = on_battlefield(&e, p1, index::STRIP_MINE).unwrap();
    let Pending::Priority { legal, .. } = e.pending().clone() else {
        unreachable!()
    };
    assert!(
        legal.abilities.contains(&(strip_mine, 1)),
        "Strip Mine's destroy ability is not named, so it stays: {:?}",
        legal.abilities
    );
    assert!(
        !legal.abilities.contains(&(wasteland, 1)),
        "while the named Wasteland's is prohibited"
    );
    assert!(
        legal.abilities.contains(&(wasteland, 0)),
        "its printed mana ability is not"
    );
}
