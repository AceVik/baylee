//! `cards/lands/phyrexian_tower.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Phyrexian Tower: "{T}: Add {C}." and "{T}, Sacrifice a creature: Add
/// {B}{B}."
///
/// Two printed mana abilities on one permanent, and they share a `{T}`. That
/// is what this card proves and no other board can: the second line names no
/// creature, so the engine has to stop and ask which one (CR 601.2h), and the
/// moment it is paid the first line must be gone from the offer, because the
/// Tower is tapped.
///
/// Reading the card cannot replace it. The printed sentences say nothing
/// about *whose* creature may be eaten or whether a land counts as one, and
/// the answer to both comes from the rules rather than the card: CR 701.21a
/// lets a player sacrifice only a permanent they control, so the opponent's
/// Elf is not on the list, and the Tower itself is a land and not fodder for
/// its own mouth. `Filter::YOUR_CREATURE` in the cost would be satisfied by a
/// list built any number of wrong ways; this is the list the player is handed.
///
/// The question arrives as `Pending::ChooseCards` with
/// `ChoicePrompt::CostSacrifice`, which is asserted here rather than the
/// options alone: choosing what to sacrifice is not targeting (CR 115.1), and
/// the prompt is the only thing that tells a client this is a cost being paid
/// and not a search. A test that read the list and ignored the label would
/// pass against `ChoicePrompt::Generic`.
///
/// The counter-halves, most of them the ones this test was born with. Two
/// Elves stand under the Tower, so a two-entry list is the filter answering
/// and not an empty table; the Tower is absent from `legal.mana_abilities`,
/// because it prints no basic land type and is no CR 305.6 shortcut; the
/// `{B}{B}` is read out of the pool immediately after the answer, which is
/// where a mana ability puts it (CR 605.3b), and no `{C}` is there beside it,
/// so it is the second line that was activated and not the first. Then both
/// lines are pressed by hand against the tapped Tower and both are refused,
/// with the board still at priority — nobody was shown a sacrifice question
/// for an activation that cannot happen, which is the order `start_activation`
/// promises and nothing else pins.
#[allow(clippy::too_many_lines)] // one board read end to end: the offer, the question, the spent {T}
#[test]
fn the_tower_eats_one_creature_and_the_shared_tap_closes_both_lines() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(313, swamp())
        .battlefield(0, &[phyrexian_tower(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let tower = on_battlefield(&engine, p0, phyrexian_tower()).expect("the Tower is on the table");
    let fodder = mine(&engine, p0, llanowar_elves(), Zone::Battlefield);
    assert_eq!(
        fodder.len(),
        2,
        "two creatures stand beside the Tower, so the two-entry list below is \
         the filter answering and not an empty table"
    );
    let theirs = mine(&engine, p1, llanowar_elves(), Zone::Battlefield);
    assert_eq!(
        theirs.len(),
        1,
        "and the opponent has one of their own, so its absence below is a rule \
         and not a missing creature"
    );

    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected a quiet main phase, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "and it is the Tower's controller who holds it");
    assert!(
        !legal.mana_abilities.contains(&tower),
        "the Tower prints no basic land type, so it is no CR 305.6 shortcut: \
         its mana belongs to `legal.abilities` and not to this list: {:?}",
        legal.mana_abilities
    );

    let offered: Vec<u32> = legal
        .abilities
        .iter()
        .filter(|(source, _)| *source == tower)
        .map(|(_, index)| *index)
        .collect();
    assert_eq!(
        offered.as_slice(),
        [0, 1],
        "both printed lines are offered: the Tower is untapped, so `{{T}}: Add \
         {{C}}` is payable, and two creatures stand under it, so `{{T}}, \
         Sacrifice a creature: Add {{B}}{{B}}` is too. `[0]` means the \
         sacrifice cost is being refused for the board instead of asked \
         about. Got: {offered:?}"
    );

    // CR 601.2h: the cost names no creature, so the player is asked which.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: tower,
                ability_index: 1,
            },
        )
        .expect("the sacrifice line is offered, so it activates");
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "paying `Sacrifice a creature` asks which one, and the engine is \
             at {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the question goes to the activating player");
    assert_eq!((min, max), (1, 1), "one creature, no more and no fewer");
    assert_eq!(
        prompt,
        ChoicePrompt::CostSacrifice,
        "a cost is not a search (CR 115.1), and the prompt is the only thing \
         that says so to a client"
    );
    assert_eq!(
        options.len(),
        2,
        "both Elves under the Tower may be eaten, and nothing else: {options:?}"
    );
    assert!(
        options.contains(&fodder[0]) && options.contains(&fodder[1]),
        "each of the controller's own creatures is on the list: {options:?}"
    );
    assert!(
        !options.contains(&tower),
        "the Tower is a land, so it is no creature to feed itself: {options:?}"
    );
    assert!(
        !options.contains(&theirs[0]),
        "and CR 701.21a lets a player sacrifice only what they control, so the \
         opponent's Elf is not on the menu: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder[0]],
            },
        )
        .expect("the answer names a creature the engine itself offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        2,
        "a mana ability resolves without the stack (CR 605.3b), so the \
         {{B}}{{B}} is in the pool the moment the cost is answered"
    );
    assert_eq!(
        pool.available(ManaColor::Colorless),
        0,
        "and it is the second line that was activated, not the first"
    );
    assert!(
        is_tapped(&engine, tower),
        "the {{T}} half of the cost was paid"
    );
    assert_eq!(
        mine(&engine, p0, llanowar_elves(), Zone::Battlefield).len(),
        1,
        "exactly one Elf was eaten for it"
    );
    assert_eq!(
        mine(&engine, p0, llanowar_elves(), Zone::Graveyard).len(),
        1,
        "and it is in its owner's graveyard, which is where a sacrifice puts it"
    );
    assert_eq!(
        mine(&engine, p1, llanowar_elves(), Zone::Battlefield).len(),
        1,
        "while the opponent's creature never moved"
    );

    // The shared {T} is spent, so both lines are gone — not merely the one
    // that was activated.
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!(
            "a mana ability hands priority straight back, and the engine is at \
             {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "and to the same player");
    let after: Vec<u32> = legal
        .abilities
        .iter()
        .filter(|(source, _)| *source == tower)
        .map(|(_, index)| *index)
        .collect();
    assert!(
        after.is_empty(),
        "both printed lines cost `{{T}}` and the Tower is tapped, so neither is \
         offered again this turn. Got: {after:?}"
    );

    for index in [0, 1] {
        assert!(
            engine
                .apply(
                    p0,
                    PlayerAction::ActivateAbility {
                        source: tower,
                        ability_index: index,
                    }
                )
                .is_err(),
            "pressing line {index} by hand is refused too, so it is unreachable \
             rather than merely unlisted"
        );
        assert!(
            matches!(engine.pending(), Pending::Priority { .. }),
            "and the refusal of line {index} asks nothing: a sacrifice question \
             is put only after the cost is known to be payable, so nobody is \
             made to give up a creature for an activation that cannot happen"
        );
    }
    assert_eq!(
        mine(&engine, p0, llanowar_elves(), Zone::Battlefield).len(),
        1,
        "and the surviving Elf survived the two refusals as well"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        0,
        "with no {{C}} squeezed out of a tapped Tower"
    );
}
