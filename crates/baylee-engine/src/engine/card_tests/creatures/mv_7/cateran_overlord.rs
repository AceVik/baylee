//! `cards/creatures/mv_7/cateran_overlord.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn cateran_overlord() -> CardIndex {
    card_index("c2223cdf-419d-4153-9551-705557461664")
}

fn sell_sword_brute() -> CardIndex {
    card_index("7549656b-359f-467e-90ab-6b21d93c26b9")
}

/// Cateran Overlord — a 7/5 for {4}{B}{B}{B}.
#[test]
fn cateran_overlord_costs_seven_black() {
    let mut engine = table7(&lots(swamp(), 7), &[cateran_overlord()], &[]);
    let lord = cast_and_resolve(&mut engine, cateran_overlord());
    assert_body(&engine, lord, (7, 5), KeywordSet::default());
}

/// "Sacrifice a creature: Regenerate this creature." The Ogre is sacrificed
/// for the shield, and the shield saves the Overlord from destruction.
#[test]
fn cateran_overlord_sacrifices_a_creature_to_regenerate() {
    let p0 = PlayerId::new(0);
    let mut engine = table7(&[cateran_overlord(), gray_ogre()], &[], &[]);
    let lord = on_battlefield(&engine, p0, cateran_overlord()).expect("out");
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("out");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: lord,
                ability_index: 1,
            },
        )
        .expect("the ability is offered");
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("the sacrifice is a choice, got {:?}", engine.pending())
    };
    assert!(options.contains(&ogre), "the Ogre may be sacrificed");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ogre],
            },
        )
        .expect("sacrificing the Ogre pays");
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert!(
        in_graveyard(&engine, p0, gray_ogre()).is_some(),
        "the Ogre was the price"
    );
    assert_eq!(
        engine
            .state()
            .object(lord)
            .expect("out")
            .regeneration_shields,
        1,
        "and a shield stands"
    );
    kill(&mut engine, lord);
    assert_eq!(on_battlefield(&engine, p0, cateran_overlord()), Some(lord));
    assert!(is_tapped(&engine, lord));
}

/// "{6}, {T}: Search your library for a Mercenary permanent card with mana
/// value 6 or less, put it onto the battlefield, then shuffle." The library
/// is all Sell-Sword Brutes, which are Mercenaries of mana value 2.
#[test]
fn cateran_overlord_fetches_a_mercenary() {
    let p0 = PlayerId::new(0);
    let mut board = lots(swamp(), 6);
    board.push(cateran_overlord());
    let mut engine = Duel::new(SEED, sell_sword_brute())
        .battlefield(0, &board)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert!(on_battlefield(&engine, p0, sell_sword_brute()).is_none());
    tap_all_mana_but(&mut engine, p0, Some(cateran_overlord()));
    activate(&mut engine, p0, cateran_overlord(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!("the pass stopped here")
    };
    assert!(!options.is_empty(), "Mercenaries are on offer");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("fetching one Mercenary");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, sell_sword_brute()).is_some(),
        "the Mercenary entered the battlefield"
    );
    assert!(
        is_tapped(
            &engine,
            on_battlefield(&engine, p0, cateran_overlord()).expect("out")
        ),
        "{{T}} was part of the cost"
    );
}
