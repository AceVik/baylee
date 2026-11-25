//! `cards/instants/mv_2/barrin_s_unmaking.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn barrin_s_unmaking() -> CardIndex {
    card_index("bd7ad427-11d9-40d1-b292-71f985711665")
}

fn begin_barrin(engine: &mut Engine<RegistryLookup>) {
    lk::float(engine, P0, &[(ManaColor::Blue, 2)]);
    cast_with_floating(engine, P0, barrin_s_unmaking());
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
}

#[test]
fn barrin_targets_any_permanent_but_returns_only_a_most_common_color() {
    for (target_card, owner, returns) in [
        (index::SAVANNAH_LIONS, P1, true),
        (index::GRIZZLY_BEARS, P0, false),
        (index::BRONZE_HORSE, P0, false),
    ] {
        let mut engine = lk::game(
            SEED,
            &[
                index::SAVANNAH_LIONS,
                index::GRIZZLY_BEARS,
                index::BRONZE_HORSE,
            ],
            &[index::SAVANNAH_LIONS],
            &[barrin_s_unmaking()],
            &[],
        );
        let target = lk::obj(&engine, owner, target_card);
        begin_barrin(&mut engine);
        let Pending::ChooseTargets {
            options, min, max, ..
        } = engine.pending().clone()
        else {
            unreachable!("Barrin's target question was just reached")
        };
        assert_eq!((min, max), (1, 1));
        assert!(
            options.contains(&target),
            "an off-color or colorless permanent is still a legal target: {options:?}"
        );
        aim_at(&mut engine, P0, target);
        lk::settle(&mut engine);

        assert_eq!(
            engine.state().object(target).unwrap().zone,
            if returns {
                baylee_engine::zone::Zone::Hand
            } else {
                baylee_engine::zone::Zone::Battlefield
            },
            "{target_card:?} with two white permanents and one green permanent"
        );
        assert_eq!(in_hand(&engine, owner, target_card).is_some(), returns);
        if returns {
            assert!(
                in_hand(&engine, P0, target_card).is_none(),
                "the opponent's permanent returns to its owner's hand"
            );
        }
    }
}

#[test]
fn barrin_accepts_every_color_tied_for_most_common() {
    for (target_card, owner) in [(index::SAVANNAH_LIONS, P0), (index::GRIZZLY_BEARS, P1)] {
        let mut engine = lk::game(
            SEED,
            &[index::SAVANNAH_LIONS],
            &[index::GRIZZLY_BEARS],
            &[barrin_s_unmaking()],
            &[],
        );
        let target = lk::obj(&engine, owner, target_card);
        begin_barrin(&mut engine);
        aim_at(&mut engine, P0, target);
        lk::settle(&mut engine);

        assert_eq!(
            engine.state().object(target).unwrap().zone,
            baylee_engine::zone::Zone::Hand,
            "white and green are tied at one permanent each"
        );
        assert!(in_hand(&engine, owner, target_card).is_some());
    }
}

#[test]
fn barrin_recounts_colors_when_it_resolves_after_an_answer() {
    let mut engine = lk::game(
        SEED,
        &[
            index::SAVANNAH_LIONS,
            index::GRIZZLY_BEARS,
            index::GRIZZLY_BEARS,
        ],
        &[index::SAVANNAH_LIONS],
        &[barrin_s_unmaking(), unsummon()],
        &[],
    );
    let target = lk::obj(&engine, P0, index::SAVANNAH_LIONS);
    let other_white = lk::obj(&engine, P1, index::SAVANNAH_LIONS);

    begin_barrin(&mut engine);
    aim_at(&mut engine, P0, target);
    assert!(!stack_is_empty(&engine), "Barrin waits on the stack");

    lk::float(&mut engine, P0, &[(ManaColor::Blue, 1)]);
    cast_with_floating(&mut engine, P0, unsummon());
    aim_at(&mut engine, P0, other_white);
    pass_until(&mut engine, |e| {
        in_hand(e, P1, index::SAVANNAH_LIONS).is_some()
    });
    assert!(
        !stack_is_empty(&engine),
        "Unsummon resolves first and Barrin is still waiting"
    );

    lk::settle(&mut engine);
    assert_eq!(
        engine.state().object(target).unwrap().zone,
        baylee_engine::zone::Zone::Battlefield,
        "white and green were tied when cast, but green leads two to one on resolution"
    );
    assert!(in_hand(&engine, P0, index::SAVANNAH_LIONS).is_none());
}
