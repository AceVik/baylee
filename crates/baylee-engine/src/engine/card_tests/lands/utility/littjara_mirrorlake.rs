//! `cards/lands/utility/littjara_mirrorlake.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The copy enters with its additional counter; only your creatures are legal targets.
#[test]
fn littjara_mirrorlake_enters_tapped_and_creates_creature_copy() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                island(),
                island(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[littjara_mirrorlake()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let lake = play_land(&mut engine, p0, littjara_mirrorlake());
    assert!(
        entered_tapped(&engine, lake),
        "littjara mirrorlake enters tapped"
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, lake));

    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).expect("controlled elf");
    let opp_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("opponent elf");

    // Float {{2}}{{G}}{{G}}{{U}} while keeping Littjara Mirrorlake untapped —
    // six, because the Elves tap for their own {{G}} beside the lands.
    tap_mana_except(&mut engine, p0, lake);
    assert_eq!(engine.state().players[0].mana_pool.total(), 6);

    activate(&mut engine, p0, littjara_mirrorlake(), 1);

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(
        options.contains(&my_elf),
        "controlled creature is a legal target"
    );
    assert!(
        !options.contains(&opp_elf),
        "opponent's creature is not a legal target"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![my_elf],
                players: vec![],
            },
        )
        .unwrap();

    assert!(
        in_graveyard(&engine, p0, littjara_mirrorlake()).is_some(),
        "littjara mirrorlake sacrificed as cost"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "{{2}}{{G}}{{G}}{{U}} out of six leaves the Elves' own mana standing"
    );

    pass_until(&mut engine, stack_is_empty);

    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one copy token created");
    assert_eq!(
        pt(&engine, tokens[0]),
        (2, 2),
        "a copied Elf with one additional counter"
    );
    assert_eq!(
        counters_on(
            &engine,
            tokens[0],
            CounterKind::Plus {
                power: 1,
                toughness: 1,
            }
        ),
        1,
        "the copy enters with an additional +1/+1 counter"
    );
}

#[test]
fn mirrorlake_doubles_tokens_and_entry_counters_but_counters_are_not_copiable() {
    use crate::engine::card_tests::legends_kit as kit;
    use baylee_core::generated::index;
    let mut e = kit::game(
        982,
        &[
            littjara_mirrorlake(),
            llanowar_elves(),
            index::DOUBLING_SEASON,
        ],
        &[],
        &[index::PHANTASMAL_IMAGE],
        &[],
    );
    let elf = kit::obj(&e, kit::P0, llanowar_elves());
    crate::replacement::put_counters(&mut e.state, elf, CounterKind::P1P1, 3);
    kit::float(
        &mut e,
        kit::P0,
        &[
            (ManaColor::Green, 2),
            (ManaColor::Blue, 1),
            (ManaColor::Colorless, 2),
        ],
    );
    kit::use_ability(&mut e, kit::P0, littjara_mirrorlake(), 1, &[elf], &[]);
    let copies = tokens_of(&e, kit::P0);
    assert_eq!(copies.len(), 2);
    for copy in &copies {
        assert_eq!(counters_on(&e, *copy, CounterKind::P1P1), 2);
        assert_eq!(pt(&e, *copy), (3, 3));
    }
    assert_eq!(counters_on(&e, elf, CounterKind::P1P1), 6);
    kit::float(&mut e, kit::P0, &[(ManaColor::Blue, 2)]);
    let image = in_hand(&e, kit::P0, index::PHANTASMAL_IMAGE).unwrap();
    cast_with_floating(&mut e, kit::P0, index::PHANTASMAL_IMAGE);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    e.apply(
        kit::P0,
        PlayerAction::ChooseObjects {
            objects: vec![copies[0]],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, image), (1, 1));
    assert_eq!(counters_on(&e, image, CounterKind::P1P1), 0);
}

/// "Activate only as a sorcery": with the cost floating and a creature to
/// copy, the ability is not offered with a spell on the stack or in combat,
/// and is offered in a main phase with an empty stack.
#[test]
fn mirrorlake_copy_ability_is_offered_only_as_a_sorcery() {
    use crate::engine::card_tests::legends_kit as kit;
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let mut e = kit::game(
        983,
        &[littjara_mirrorlake(), llanowar_elves()],
        &[],
        &[index::DARK_RITUAL],
        &[],
    );
    let price = [
        (ManaColor::Green, 2),
        (ManaColor::Blue, 1),
        (ManaColor::Colorless, 2),
    ];
    kit::float(&mut e, p0, &[(ManaColor::Black, 1)]);
    cast_with_floating(&mut e, p0, index::DARK_RITUAL);
    assert!(!stack_is_empty(&e), "Dark Ritual waits on the stack");
    kit::float(&mut e, p0, &price);
    assert!(
        !kit::offered(&e, littjara_mirrorlake(), p0, 1),
        "\"Activate only as a sorcery\": not with a spell on the stack, though {{2}}{{G}}{{G}}{{U}} floats"
    );

    pass_until(&mut e, stack_is_empty);
    assert!(
        kit::offered(&e, littjara_mirrorlake(), p0, 1),
        "main phase, empty stack, cost floating: offered"
    );

    kit::to_step(&mut e, p0, crate::turn::Step::CombatBegin);
    kit::float(&mut e, p0, &price);
    assert!(
        !kit::offered(&e, littjara_mirrorlake(), p0, 1),
        "\"Activate only as a sorcery\": not in combat"
    );
}
