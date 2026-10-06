//! `cards/lands/gates/talon_gates_of_madara.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Talon Gates of Madara is a Gate that enters untapped, phases a creature
/// out as it arrives, taps for {C}, and for {1} plus its own {T} makes one
/// mana of any colour — the `{4}` that would put it onto the battlefield from
/// hand is the `Coverage::Partial` gap and nothing here touches it. The entry
/// trigger is aimed at one of two *identical* Elves across the table, so the
/// Elf it did not name is the control that says "up to one target creature" is
/// a target and not a board sweep, and `min` 0 is what makes the "up to" an
/// offer rather than an obligation.
///
/// Both printed mana lines are pressed in one turn because a land taps once:
/// the second Gate was *placed* by the harness rather than played, so it never
/// triggered its own entry clause and is still standing when the played one has
/// paid its own `{T}`. The Forest is the `{1}`, which is what the any-colour
/// line proves by cost — its green leaves the pool and only the named colour
/// arrives.
#[test]
#[allow(clippy::too_many_lines)] // one land's whole printed text, clause by clause
fn talon_gates_of_madara_phases_one_creature_out_and_taps_for_both_of_its_manas() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), talon_gates_of_madara()])
        .hand(0, &[talon_gates_of_madara()])
        .battlefield(1, &[llanowar_elves(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    // A seed puts either seat on the play, and this walk crosses whatever
    // turn that is rather than assuming p0's first main is the first one.
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = all_on_battlefield(&engine, p1, llanowar_elves());
    assert_eq!(elves.len(), 2, "two identical Elves across the table");
    let (doomed, bystander) = (elves[0], elves[1]);
    assert!(
        !engine
            .state()
            .object(doomed)
            .expect("the Elf is an object")
            .status
            .contains(Status::PHASED_OUT),
        "nothing has phased anything out yet"
    );

    let played = play_land(&mut engine, p0, talon_gates_of_madara());
    assert!(
        !is_tapped(&engine, played),
        "a Gate with no enters-tapped clause arrives standing up"
    );

    // The entry trigger targets when it is put on the stack (CR 603.3d), so
    // the question comes before anything resolves.
    let mut asked = false;
    for _ in 0..20 {
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player,
                options,
                min,
                max,
                ..
            } => {
                assert_eq!(player, p0, "the Gate's controller picks the victim");
                assert_eq!((min, max), (0, 1), "\"up to one target creature\"");
                assert!(
                    options.contains(&doomed) && options.contains(&bystander),
                    "both creatures are legal targets: {options:?}"
                );
                assert_eq!(options.len(), 2, "and no land is one: {options:?}");
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseTargets {
                            objects: vec![doomed],
                            players: Vec::new(),
                        },
                    )
                    .unwrap();
                asked = true;
            }
            Pending::Priority { .. } if stack_is_empty(&engine) => break,
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the entry trigger resolves: {other:?}"),
        }
    }
    assert!(asked, "the trigger asks which creature before it resolves");
    assert!(stack_is_empty(&engine), "and the stack is clear again");

    assert!(
        engine
            .state()
            .object(doomed)
            .expect("a phased-out permanent is still an object")
            .status
            .contains(Status::PHASED_OUT),
        "\"when this land enters, up to one target creature phases out\""
    );
    assert!(
        !engine
            .state()
            .object(bystander)
            .expect("the Elf is an object")
            .status
            .contains(Status::PHASED_OUT),
        "the Elf the trigger was not aimed at is untouched, so the clause is \
         a target and not a board sweep"
    );

    // The one Forest is the {1}. Both Gates are kept back: their two mana
    // lines are exactly what `tap_all_mana` would take, and this test presses
    // them by hand — the second one because the {1} has to be floating first.
    tap_all_mana_but(&mut engine, p0, Some(talon_gates_of_madara()));
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "the Forest, and no more"
    );
    assert_eq!(pool.total(), 1, "one mana in the pool");
    assert_eq!(
        all_on_battlefield(&engine, p0, talon_gates_of_madara())
            .iter()
            .copied()
            .filter(|id| !is_tapped(&engine, *id))
            .count(),
        2,
        "and both Gates are still standing, which is what that tap bought"
    );

    // Ability 2: "{1}, {T}: Add one mana of any color."
    activate(&mut engine, p0, talon_gates_of_madara(), 2);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!("\"any color\" is a question, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat that activated names the colour");
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert!(
            options.contains(&color),
            "\"any color\" includes {color:?}: {options:?}"
        );
    }
    assert_eq!(
        options.len(),
        5,
        "the five colors, and colorless is no color at all (CR 105.4): {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the colours it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "and the Forest's green paid the {{1}} — the cost is a cost, not a label"
    );
    assert_eq!(pool.total(), 1, "one mana in, one mana out");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    let gates = all_on_battlefield(&engine, p0, talon_gates_of_madara());
    assert_eq!(
        gates
            .iter()
            .copied()
            .filter(|id| is_tapped(&engine, *id))
            .count(),
        1,
        "the {{T}} in that price tapped the Gate that paid it: {gates:?}"
    );

    // Ability 1: the plain "{T}: Add {C}", off the Gate the first line left
    // standing — which is the only reason the offer still carries it.
    activate(&mut engine, p0, talon_gates_of_madara(), 1);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "\"{{T}}: Add {{C}}\", from the Gate rather than from the tapped Forest"
    );
    assert_eq!(pool.total(), 2, "a black mana and a colorless one");
    assert_eq!(
        all_on_battlefield(&engine, p0, talon_gates_of_madara())
            .iter()
            .copied()
            .filter(|id| !is_tapped(&engine, *id))
            .count(),
        0,
        "both Gates are down now, one for each printed line"
    );
}
