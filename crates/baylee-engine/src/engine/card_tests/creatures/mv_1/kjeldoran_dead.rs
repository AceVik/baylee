//! `cards/creatures/mv_1/kjeldoran_dead.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn kjeldoran_dead() -> CardIndex {
    card_index("61e00946-7df7-4298-91b2-c656169e6601")
}

/// Kjeldoran Dead (61e00946-7df7-4298-91b2-c656169e6601) — {B} 3/1 Creature — Skeleton:
/// - "When this creature enters, sacrifice a creature."
/// - "{B}: Regenerate this creature."
#[test]
fn kjeldoran_dead_sacrifices_a_creature_on_entry_and_regenerates_for_one_black() {
    let mut engine = Duel::new(SEED, index::SWAMP)
        .battlefield(0, &[index::SWAMP, index::SWAMP, llanowar_elves()])
        .hand(0, &[kjeldoran_dead()])
        .battlefield(1, &[index::MOUNTAIN, index::GRIZZLY_BEARS])
        .hand(1, &[index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let elf = on_battlefield(&engine, P0, llanowar_elves()).expect("Elf seated");
    let their_bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).expect("Bears seated");

    // Tap one Swamp to cast Kjeldoran Dead.
    let swamp0 = on_battlefield(&engine, P0, index::SWAMP).unwrap();
    engine
        .apply(P0, PlayerAction::ActivateManaAbility { source: swamp0 })
        .unwrap();
    cast_from_hand(&mut engine, P0, kjeldoran_dead());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let dead = on_battlefield(&engine, P0, kjeldoran_dead()).expect("Kjeldoran Dead entered");
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!()
    };
    assert!(
        options.contains(&elf) && options.contains(&dead),
        "ETB sacrifice offers any creature P0 controls (both Llanowar Elves and Kjeldoran Dead)"
    );
    assert!(
        !options.contains(&their_bears),
        "'sacrifice a creature' offers only creatures its controller has"
    );
    engine
        .apply(P0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, P0, llanowar_elves()).is_some(),
        "Llanowar Elves was sacrificed to Kjeldoran Dead's ETB trigger"
    );
    assert!(on_battlefield(&engine, P0, kjeldoran_dead()).is_some());
    assert!(
        on_battlefield(&engine, P1, index::GRIZZLY_BEARS).is_some(),
        "the opponent's creature is not sacrificed"
    );

    // P0 activates "{B}: Regenerate this creature", then P1 casts Lightning Bolt targeting Kjeldoran Dead.
    tap_all_mana_but(&mut engine, P0, None);
    activate(&mut engine, P0, kjeldoran_dead(), 1);
    pass_until(&mut engine, stack_is_empty);

    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    tap_all_mana_but(&mut engine, P1, None);
    cast_from_hand(&mut engine, P1, index::LIGHTNING_BOLT);
    aim_at(&mut engine, P1, dead);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P0, kjeldoran_dead()).is_some(),
        "regeneration shield saves Kjeldoran Dead from lethal Lightning Bolt"
    );
    assert!(
        is_tapped(&engine, dead),
        "regenerating taps Kjeldoran Dead and clears its damage"
    );
    assert_eq!(engine.state().object(dead).unwrap().damage, 0);
}
