//! `cards/creatures/mv_2/giada_font_of_hope.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn giada() -> CardIndex {
    card_index("48e6d3d8-2f27-4017-acdd-40bce8cdbc02")
}

fn inspiring_overseer() -> CardIndex {
    card_index("d646e42b-5635-4798-b633-29c093b66a55")
}

fn youthful_valkyrie() -> CardIndex {
    card_index("6d37ba4b-ff56-4eec-9dc2-2d7f357dc9c9")
}

fn counters(engine: &Engine<RegistryLookup>, id: ObjectId) -> u16 {
    engine
        .state()
        .object(id)
        .unwrap()
        .counters
        .get(CounterKind::P1P1)
}

#[test]
fn giada_counts_angels_already_controlled_and_excludes_herself_and_other_permanents() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                giada(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                forest(),
                forest(),
            ],
        )
        .battlefield(1, &[inspiring_overseer()])
        .hand(
            0,
            &[inspiring_overseer(), inspiring_overseer(), grizzly_bears()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);

    let font = on_battlefield(&engine, P0, giada()).unwrap();
    assert_eq!(counters(&engine, font), 0, "Giada is not another Angel");

    // The opponent's Angel does not count. Giada alone does, so the Overseer
    // arrives with exactly one additional counter.
    cast_from_hand(&mut engine, P0, inspiring_overseer());
    pass_until(&mut engine, stack_is_empty);
    let angel = on_battlefield(&engine, P0, inspiring_overseer()).unwrap();
    assert_eq!(counters(&engine, angel), 1);
    assert_eq!(pt(&engine, angel), (3, 2));

    // A second Angel counts both Angels already on its controller's side.
    let second = in_hand(&engine, P0, inspiring_overseer()).unwrap();
    cast_from_hand(&mut engine, P0, inspiring_overseer());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(counters(&engine, second), 2);

    // Non-Angels receive no counters, even though their controller has three.
    cast_from_hand(&mut engine, P0, grizzly_bears());
    pass_until(&mut engine, stack_is_empty);
    let bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    assert_eq!(counters(&engine, bears), 0);
}

#[test]
fn giada_mana_casts_an_angel_but_cannot_pay_for_a_non_angel() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[giada(), plains()])
        .hand(0, &[youthful_valkyrie(), ondu_cleric()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);

    tap_all_mana(&mut engine, P0);
    let offer = priority_offer(&engine);
    assert!(
        offer
            .castable
            .contains(&in_hand(&engine, P0, youthful_valkyrie()).unwrap())
    );
    let non_angel = in_hand(&engine, P0, ondu_cleric()).unwrap();
    assert_eq!(
        crate::casting::spendable_units(
            engine.state(),
            P0,
            crate::casting::SpendFor::Spell(non_angel),
        ),
        1,
        "only the Plains is available to the non-Angel",
    );
    assert!(
        !offer.castable.contains(&non_angel),
        "the restricted white mana cannot pay the generic part of a non-Angel spell"
    );
    cast_with_floating(&mut engine, P0, youthful_valkyrie());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}

#[test]
fn giada_flies_and_attacks_without_tapping() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[giada()])
        .battlefield(1, &[llanowar_elves(), baleful_strix()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);

    let font = on_battlefield(&engine, P0, giada()).unwrap();
    let ground = on_battlefield(&engine, P1, llanowar_elves()).unwrap();
    let flying = on_battlefield(&engine, P1, baleful_strix()).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(font, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(
        blockers
            .iter()
            .find(|option| option.blocker == ground)
            .is_none_or(|option| !option.attackers.contains(&font))
    );
    assert!(
        blockers
            .iter()
            .find(|option| option.blocker == flying)
            .is_some_and(|option| option.attackers.contains(&font))
    );

    pass_until(&mut engine, |e| e.state().turn.phase == Phase::Ending);
    assert_eq!(engine.state().players[1].life, 18);
    assert!(!is_tapped(&engine, font), "vigilance kept Giada untapped");
}
