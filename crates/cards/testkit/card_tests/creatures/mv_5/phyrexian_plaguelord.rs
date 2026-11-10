//! `cards/creatures/mv_5/phyrexian_plaguelord.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "aff9e844-9e03-490b-b44f-10d385738cc6"

/// Phyrexian Plaguelord prints two activated abilities and neither of them is
/// paid for with mana: "{T}, Sacrifice this creature: Target creature gets
/// -4/-4 until end of turn", and "Sacrifice a creature: Target creature gets
/// -1/-1 until end of turn".
///
/// The board reads both, and every clause either one turns on. The five Swamps
/// pay `{3}{B}{B}` down to the last mana, so neither activation is paid for
/// with anything but permanents; the Llanowar Elves are the creature the
/// -1/-1 line eats while the same line aimed at the Plaguelord itself reads
/// -1/-1 off the body the card prints (4/4 to 3/3); and the turn that follows
/// hands that body back unharmed, which is the "until end of turn" no board
/// can see inside the turn the card was cast in. The -4/-4 is aimed across the
/// table because "target creature" names no side, and it is measured against
/// the body it lands on rather than assumed: what the ability prints is the
/// -4/-4, not the creature wearing it.
#[test]
#[allow(clippy::too_many_lines)]
fn phyrexian_plaguelord_sells_a_creature_for_minus_one_and_itself_for_minus_four() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[phyrexian_plaguelord()])
        .battlefield(1, &[sliver_queen()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Five Swamps pay {3}{B}{B} down to the last mana, and the Elf is named as
    // the printing kept back: it is the creature the second line is about to
    // eat, and `tap_all_mana` would have drunk its own `{T}: Add {G}` too.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five tapped Swamps, and the Elf contributed nothing"
    );
    cast_with_floating(&mut engine, p0, phyrexian_plaguelord());
    pass_until(&mut engine, stack_is_empty);

    let lord =
        on_battlefield(&engine, p0, phyrexian_plaguelord()).expect("the Plaguelord resolved");
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    let theirs = on_battlefield(&engine, p1, sliver_queen()).expect("their body is out");
    assert_eq!(pt(&engine, lord), (4, 4), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{3}}{{B}}{{B}} cost the whole pool, so neither ability below is paid \
         for with mana"
    );

    // The second printed line, "Sacrifice a creature: Target creature gets
    // -1/-1 until end of turn". Both questions are answered in whichever order
    // they arrive — which creature is aimed at (CR 601.2c) and which is being
    // given up (CR 601.2h) — and each menu is read where it is published.
    activate(&mut engine, p0, phyrexian_plaguelord(), 1);
    let mut aimed: Option<Vec<ObjectId>> = None;
    let mut menu: Option<Vec<ObjectId>> = None;
    for _ in 0..6 {
        if aimed.is_some() && menu.is_some() {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player,
                options,
                min,
                max,
                ..
            } if aimed.is_none() => {
                assert_eq!(player, p0, "the activating seat is the one that aims it");
                assert_eq!(
                    (min, max),
                    (1, 1),
                    "one creature, and the ability asks once"
                );
                aimed = Some(options);
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![lord],
                        },
                    )
                    .expect("the Plaguelord is a creature, and the question offered it");
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
                ..
            } if menu.is_none() => {
                assert_eq!(player, p0, "the activating seat pays its own cost");
                assert_eq!(
                    prompt,
                    ChoicePrompt::CostSacrifice,
                    "the variant is what tells a client this is a cost and not a search"
                );
                assert_eq!((min, max), (1, 1), "one creature, no more and no fewer");
                menu = Some(options);
                engine
                    .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
                    .expect("the Elf is the creature the question offered");
            }
            other => panic!("unexpected while the sacrifice ability is paid: {other:?}"),
        }
    }

    let aimed = aimed.expect("\"target creature\" is a target choice");
    let menu = menu.expect("the cost asks which creature is given up");
    assert!(
        aimed.contains(&lord) && aimed.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {aimed:?}"
    );
    assert!(
        menu.contains(&elf) && menu.contains(&lord),
        "a creature you control is the whole menu, and the Plaguelord is one too: {menu:?}"
    );
    assert!(
        !menu.contains(&theirs),
        "CR 701.21a: an opponent's creature is not yours to sacrifice: {menu:?}"
    );

    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, lord),
        (3, 3),
        "-1/-1 on the body the card prints, because the ability named the \
         Plaguelord itself"
    );
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "the creature the cost named is in its owner's graveyard"
    );
    assert!(
        !is_tapped(&engine, lord),
        "this line's whole price is a creature: the tap symbol belongs to the other"
    );

    // Across the turn boundary and back, which is where the printed "until end
    // of turn" is readable: the -1/-1 lasted the turn it was given and no
    // longer.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, lord),
        (4, 4),
        "\"until end of turn\": the Plaguelord is the 4/4 it was printed as again"
    );

    // The first printed line, "{T}, Sacrifice this creature: Target creature
    // gets -4/-4 until end of turn". Targets are named at CR 601.2c and the
    // price is paid last (CR 601.2h), so while the question below stands the
    // source is still untapped and still on the battlefield.
    let before = pt(&engine, theirs);
    activate(&mut engine, p0, phyrexian_plaguelord(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that aims it");
    assert_eq!(
        (min, max),
        (1, 1),
        "one creature, and the ability asks once"
    );
    assert!(
        options.contains(&theirs) && options.contains(&lord),
        "\"target creature\" reaches both sides of the table: {options:?}"
    );
    assert!(
        on_battlefield(&engine, p0, phyrexian_plaguelord()).is_some(),
        "the sacrifice is a cost: while the question stands nothing is given up"
    );
    assert!(!is_tapped(&engine, lord), "and nothing is tapped either");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the creature across the table was one of the options");

    assert!(
        on_battlefield(&engine, p0, phyrexian_plaguelord()).is_none(),
        "\"Sacrifice this creature\" takes the whole card, tap and all"
    );
    assert!(
        in_graveyard(&engine, p0, phyrexian_plaguelord()).is_some(),
        "and a sacrificed creature goes to its owner's graveyard"
    );
    assert!(
        !stack_is_empty(&engine),
        "the -4/-4 is no mana ability, so it is waiting on the stack"
    );
    assert_eq!(
        pt(&engine, theirs),
        before,
        "and the creature it named is untouched while the ability waits"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p1, sliver_queen()).is_some(),
        "the target is still standing, so the reading below is about the pump"
    );
    assert_eq!(
        pt(&engine, theirs),
        (before.0 - 4, before.1 - 4),
        "\"Target creature gets -4/-4\": four off both halves of the body it \
         landed on"
    );
}
