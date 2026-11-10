//! `cards/lands/restricted/baxter_building.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Baxter Building is a land with `{T}: Add {C}` and `{4}, {T}: Add four mana
/// in any combination of colors`; the third printed line — draw a card, only
/// with a creature of toughness 4 or greater — is the file's
/// `Coverage::Partial` gap, so the four-mana line is what is played here.
///
/// The four Forests are the spine of the scenario and not scenery: a cost is
/// read off the *pool* (CR 601.2h), so the `{4}` line is unoffered while
/// nothing floats and offered the moment four green are in it — and once the
/// four points have been named black, white, blue and red, the pool holds one
/// of each with **no green in it at all**, a reading no Forest on the board
/// could have produced.
#[test]
#[allow(clippy::too_many_lines)] // one land, played through every clause it prints
fn baxter_building_taps_for_four_mana_in_the_colors_its_controller_names() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[baxter_building(), forest(), forest(), forest(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let building =
        on_battlefield(&engine, p0, baxter_building()).expect("the Baxter Building stands");
    assert_eq!(
        all_on_battlefield(&engine, p0, forest()).len(),
        4,
        "four Forests, which is exactly the {{4}} the second line asks for"
    );

    // Ability 1 is `{4}, {T}: Add four mana…`; ability 0 is the `{T}: Add {C}`
    // this test keeps out of the pool.
    let offered = |engine: &Engine<RegistryLookup>| -> bool {
        matches!(
            engine.pending(),
            Pending::Priority { legal, .. } if legal.abilities.contains(&(building, 1))
        )
    };
    assert!(
        !offered(&engine),
        "nothing floats, so {{4}} is unpayable and the line is not offered \
         however many untapped Forests stand beside it"
    );

    // The four Forests, with the Building named as the one thing kept back:
    // its own `{T}` is the price paid by index just below, and a source
    // tapped for mana could not pay it.
    tap_all_mana_but(&mut engine, p0, Some(baxter_building()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        4,
        "four Forests' worth of green, and the Building is still standing"
    );
    assert!(offered(&engine), "four floating mana pays the cost");

    activate(&mut engine, p0, baxter_building(), 1);

    // "In any combination of colors" is one question per point of mana, and
    // the answers here are four different ones — which is the only way the
    // pool can end up holding four colors rather than four of a kind.
    let picks = [
        ManaColor::Black,
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Red,
    ];
    let mut named: Vec<ManaColor> = Vec::new();
    for _ in 0..16 {
        match engine.pending().clone() {
            Pending::ChooseColor { player, options } => {
                assert_eq!(
                    player, p0,
                    "the activating seat is the one that names the colors"
                );
                for color in picks {
                    assert!(
                        options.contains(&color),
                        "{color:?} is one of the combinations on offer: {options:?}"
                    );
                }
                let pick = picks[named.len().min(picks.len() - 1)];
                engine
                    .apply(p0, PlayerAction::ChooseColor(pick))
                    .expect("the answer came out of the question that enumerated it");
                named.push(pick);
            }
            Pending::Priority { player, .. } => {
                assert_eq!(player, p0, "the seat holds priority again");
                break;
            }
            other => panic!("expected a color choice, got {other:?}"),
        }
    }
    assert_eq!(
        named.len(),
        4,
        "four mana in any combination is four questions, one per mana"
    );

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 4, "{{4}}, {{T}}: four mana, and no more");
    for color in picks {
        assert_eq!(
            pool.available(color),
            1,
            "one of the four points is {color:?}, because that is what was named"
        );
    }
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "the four green that paid the cost are spent, and green is a color \
         this seat never named — so none of the four came off a Forest"
    );
    assert!(
        is_tapped(&engine, building),
        "the Building paid its own {{T}}"
    );
    assert!(
        all_on_battlefield(&engine, p0, forest())
            .iter()
            .all(|id| is_tapped(&engine, *id)),
        "and the four Forests that paid the {{4}} are still tapped"
    );
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
}

/// Baxter Building's third line is the same shape read the other way round:
/// "{4}, {T}: Draw a card. Activate only if you control a creature with
/// **toughness** 4 or greater." A 1/1 fails it and a 6/6 passes it, and the
/// two mana abilities above it are the control — they cost the same `{T}`
/// and print no condition, so an empty list would mean something else.
#[test]
fn baxter_building_draws_only_while_a_four_toughness_creature_stands() {
    let p0 = PlayerId::new(0);

    let mut small = Duel::new(9203, forest())
        .battlefield(
            0,
            &[
                baxter_building(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .start();
    keep_mulligans(&mut small);
    reach_main_phase(&mut small, p0);
    let building = on_battlefield(&small, p0, baxter_building()).expect("the Building is seated");
    tap_mana_except(&mut small, p0, building);
    let Pending::Priority { legal, .. } = small.pending().clone() else {
        panic!("expected priority, got {:?}", small.pending())
    };
    assert!(
        legal.abilities.contains(&(building, 1)),
        "the {{4}} mana ability costs the same and is offered: {:?}",
        legal.abilities
    );
    assert!(
        !legal.abilities.contains(&(building, 2)),
        "a 1/1 is not \"a creature with toughness 4 or greater\""
    );

    let mut big = Duel::new(9204, forest())
        .battlefield(
            0,
            &[
                baxter_building(),
                forest(),
                forest(),
                forest(),
                forest(),
                rootbreaker_wurm(),
            ],
        )
        .start();
    keep_mulligans(&mut big);
    reach_main_phase(&mut big, p0);
    let building = on_battlefield(&big, p0, baxter_building()).expect("the Building is seated");
    tap_mana_except(&mut big, p0, building);
    let library_before = library_size(&big, p0);
    activate(&mut big, p0, baxter_building(), 2);
    pass_until(&mut big, stack_is_empty);
    assert_eq!(library_size(&big, p0), library_before - 1, "one card drawn");
}
