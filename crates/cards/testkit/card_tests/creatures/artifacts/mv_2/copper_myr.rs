//! `cards/creatures/artifacts/mv_2/copper_myr.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Copper Myr` prints `{{T}}: Add {{G}}` on a 1/1 artifact creature with `Coverage::Implemented`.
/// Seated on the battlefield from turn one, it stands untapped, unsick, and with no other mana sources
/// present. When `tap_all_mana` is called, the Myr taps for mana directly without using the stack,
/// providing exactly one green mana into seat 0's mana pool.
#[test]
fn copper_myr_taps_for_one_green_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[copper_myr()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let myr = on_battlefield(&engine, p0, copper_myr()).expect("copper myr is seated");
    assert_eq!(pt(&engine, myr), (1, 1));
    let t = types(&engine, myr);
    assert!(t.contains(TypeSet::ARTIFACT) && t.contains(TypeSet::CREATURE));
    assert!(!is_tapped(&engine, myr));
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    tap_all_mana(&mut engine, p0);

    assert!(is_tapped(&engine, myr));
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert_eq!(pool.total(), 1);
    assert!(stack_is_empty(&engine));
}

/// Declining is naming nothing, every time: a card left for one of its types
/// is still offered for the next (the artifact creature, left as an artifact,
/// is on the creature menu), and all ten go to the bottom.
#[test]
fn atraxa_grand_unifier_declined_puts_all_ten_at_the_bottom() {
    let p0 = PlayerId::new(0);
    let stack = [
        copper_myr(),
        sol_ring(),
        llanowar_elves(),
        thundering_giant(),
        counterspell(),
        mountain(),
        damn(),
        underworld_breach(),
        karn_the_great_creator(),
        swords_to_plowshares(),
    ];
    let (mut engine, ids) = atraxa_reveals(&stack);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let mut asked = Vec::new();
    while let Some((card_type, options)) = atraxa_question(&engine) {
        if card_type == TypeSet::CREATURE {
            assert!(
                options.contains(&ids[0]),
                "the Myr is still a creature on offer"
            );
        }
        asked.push(card_type);
        engine
            .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
            .unwrap();
    }
    assert_eq!(asked.len(), 7);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );
    let library = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    assert_eq!(sorted(library[..10].to_vec()), sorted(ids.clone()));
}
