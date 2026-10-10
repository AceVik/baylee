//! `cards/instants/mv_1/sleight_of_mind.rs`, played.
//!
//! The spell half (a Red Elemental Blast whose "blue" becomes "green" and so
//! loses its target) and a changed protection against an opponent's spells
//! are played in `alpha_completion_review::text_copy_review`.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

/// "Change the text of target … permanent by replacing all instances of one
/// color word with another. … This effect lasts indefinitely." Black
/// Knight's "protection from white" becomes "protection from green": a full
/// round of turns later Giant Growth cannot name it and Holy Strength can,
/// and the Knight is still called Black Knight (CR 612.2).
#[test]
fn sleight_of_mind_moves_a_permanents_protection_to_another_color_for_good() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let sleight = card_index("99dba614-40d3-41c1-a3b2-edc8777b010f");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                island(),
                plains(),
                forest(),
                index::BLACK_KNIGHT,
                index::GRIZZLY_BEARS,
            ],
        )
        .hand(0, &[sleight, giant_growth(), index::HOLY_STRENGTH])
        .battlefield(1, &[swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let knight = on_battlefield(&engine, p0, index::BLACK_KNIGHT).expect("the Knight is out");
    let bears = on_battlefield(&engine, p0, index::GRIZZLY_BEARS).expect("the Bears are out");
    let name = engine
        .state()
        .object(knight)
        .unwrap()
        .characteristics()
        .name;

    cast_from_hand(&mut engine, p0, sleight);
    aim_at(&mut engine, p0, knight);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    let Pending::ChooseNumber {
        player, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("the pair of words is asked, got {:?}", engine.pending())
    };
    assert_eq!((player, min, max), (p0, 0, 19), "twenty ordered pairs");
    // WUBRG order: white (0) -> green (4) is 0 * 4 + (4 - 1).
    engine.apply(p0, PlayerAction::ChooseNumber(3)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        engine
            .state()
            .object(knight)
            .unwrap()
            .characteristics()
            .name,
        name,
        "the name is not a color word"
    );

    cast_from_hand(&mut engine, p0, giant_growth());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "Giant Growth asks for its creature, got {:?}",
            engine.pending()
        )
    };
    assert!(
        !options.contains(&knight),
        "protection from green now: a green spell cannot target it"
    );
    assert!(options.contains(&bears));
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bears), (5, 5));

    cast_with_floating(&mut engine, p0, index::HOLY_STRENGTH);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "Holy Strength asks for its creature, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&knight),
        "no protection from white any more: a white Aura may enchant it"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![knight],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, knight), (3, 4), "a white Aura stays on it");
}
