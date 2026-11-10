//! `cards/creatures/mv_1/dwarven_armorer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Dwarven Bloodboiler costs `{R}{R}{R}` and is a 2/2 Dwarf with an
/// activated ability: "Tap an untapped Dwarf you control: Target creature
/// gets +2/+0 until end of turn." The card's text line does not answer the
/// cost itself — *which* Dwarf and whether the Elf next to it is one is only
/// in the offer —, so there are two Dwarf controls: the Armorer as the Dwarf
/// to tap, and one Llanowar Elves each below and above the table, both of
/// which are not Dwarfs. `+2/+0` on a printed 1/1 reads as `(3, 1)`; a
/// `(3, 3)` would be a boost that the card does not print.
#[test]
#[allow(clippy::too_many_lines)] // eine Aktivierung, beide Fragen und jede gedruckte Klausel daran gelesen
fn dwarven_bloodboiler_taps_a_dwarf_you_control_to_pump_a_creature_it_names() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                dwarven_armorer(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[dwarven_bloodboiler()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The {R}{R}{R} from the three Mountains: the Dwarf enters, as a
    // creature enters, and neither the Armorer nor the Elf next to it tapped
    // for it.
    cast_from_hand(&mut engine, p0, dwarven_bloodboiler());
    pass_until(&mut engine, stack_is_empty);
    let boiler =
        on_battlefield(&engine, p0, dwarven_bloodboiler()).expect("the Bloodboiler resolved");
    let dwarf = on_battlefield(&engine, p0, dwarven_armorer()).expect("the Armorer is out");
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(pt(&engine, mine), (1, 1), "a printed 1/1 before the pump");
    assert!(
        !is_tapped(&engine, dwarf),
        "the Dwarf the cost wants is untapped"
    );

    activate(&mut engine, p0, dwarven_bloodboiler(), 0);

    // Two questions, in the order the rules give them — the target at
    // CR 601.2c and the tap symbol at CR 601.2h —, answered as they
    // arrive.
    let mut aimed = false;
    let mut menu: Vec<ObjectId> = Vec::new();
    for _ in 0..12 {
        if aimed && !menu.is_empty() {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player, options, ..
            } => {
                assert_eq!(player, p0, "der aktivierende Sitz zielt selbst");
                assert!(
                    options.contains(&mine) && options.contains(&theirs),
                    "\"target creature\" is any creature, on either side of the table: \
                     {options:?}"
                );
                assert!(
                    options.contains(&boiler),
                    "and the Bloodboiler is itself a creature: {options:?}"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![mine],
                        },
                    )
                    .unwrap();
                aimed = true;
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
                ..
            } => {
                assert_eq!(player, p0, "the activating seat pays its own cost");
                assert_eq!(
                    prompt,
                    ChoicePrompt::CostTap,
                    "a cost and not a search — and the variant is everything \
                     by which a client recognizes that the response is not \
                     lost"
                );
                assert_eq!((min, max), (1, 1), "a Dwarf, no more and no less");
                menu = options;
                assert!(
                    !is_tapped(&engine, dwarf),
                    "the question stands before the cost is paid"
                );
                assert_eq!(
                    pt(&engine, mine),
                    (1, 1),
                    "and the pump only drops when the ability is resolved"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![dwarf],
                        },
                    )
                    .unwrap();
            }
            other => panic!("unerwartet zwischen den zwei Fragen einer Aktivierung: {other:?}"),
        }
    }
    assert!(aimed, "the pump has a target");
    assert!(!menu.is_empty(), "the tap cost is queried");
    assert!(
        menu.contains(&dwarf),
        "ein Zwerg unter eigener Kontrolle ist die ganze Antwort: {menu:?}"
    );
    assert!(
        !menu.contains(&mine) && !menu.contains(&theirs),
        "\"an untapped **Dwarf** you control\": no Llanowar Elves, and the one \
         across the table is not yours anyway: {menu:?}"
    );

    assert!(
        !stack_is_empty(&engine),
        "paying the cost is not resolving the ability"
    );
    assert_eq!(pt(&engine, mine), (1, 1), "also ist noch nichts gepumpt");
    assert!(is_tapped(&engine, dwarf), "the tap symbol was the cost");
    assert!(
        !is_tapped(&engine, boiler),
        "the Bloodboiler paid nothing: the cost is *another* creature"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, mine),
        (3, 1),
        "+2/+0 on the creature the ability named — (3, 3) would be a \
         boost that the card does not print"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and nothing on the creature that it did not name"
    );
}
