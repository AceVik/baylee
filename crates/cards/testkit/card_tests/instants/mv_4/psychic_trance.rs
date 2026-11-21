//! `cards/instants/mv_4/psychic_trance.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;
use baylee_engine::choice::granted_slot;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "5870175d-83df-43af-b199-fcd1e60b824e"
fn psychic_trance() -> CardIndex {
    card_index("5870175d-83df-43af-b199-fcd1e60b824e")
}

fn granted_index(engine: &Engine<RegistryLookup>, source: ObjectId) -> Option<u32> {
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    legal
        .abilities
        .iter()
        .find_map(|&(id, idx)| (id == source && granted_slot(idx).is_some()).then_some(idx))
}

/// Psychic Trance ({2}{U}{U}): until end of turn, Wizards YOU control (not
/// non-Wizards and not opponent's Wizards) gain "{T}: Counter target spell",
/// and the ability disappears on the next turn.
#[test]
fn psychic_trance_grants_counterspell_ability_only_to_your_wizards_until_eot() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                index::PRODIGAL_SORCERER,
                index::GRIZZLY_BEARS,
            ],
        )
        .hand(0, &[psychic_trance()])
        .battlefield(1, &[mountain(), index::PRODIGAL_SORCERER])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

    let my_wiz = lk::obj(&engine, P0, index::PRODIGAL_SORCERER);
    let my_bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let their_wiz = lk::obj(&engine, P1, index::PRODIGAL_SORCERER);

    // P0 casts Psychic Trance on P1's main phase.
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, psychic_trance());
    pass_until(&mut engine, stack_is_empty);

    // P1 casts Lightning Bolt at P0.
    tap_all_mana(&mut engine, P1);
    cast_with_floating(&mut engine, P1, lightning_bolt());
    lk::aim(&mut engine, &[], &[P0]);
    let bolt = on_stack(&engine, lightning_bolt()).unwrap();
    assert!(
        granted_index(&engine, their_wiz).is_none(),
        "opponent's Wizard did NOT gain the counter ability"
    );
    engine.apply(P1, PlayerAction::PassPriority).unwrap();

    // P0's Wizard has the granted ability; P0's non-Wizard Bear does not.
    assert!(
        granted_index(&engine, my_bears).is_none(),
        "non-Wizard Bear did NOT gain the counter ability"
    );
    let grant =
        granted_index(&engine, my_wiz).expect("own Wizard gained {T}: Counter target spell");
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: my_wiz,
                ability_index: grant,
            },
        )
        .unwrap();
    lk::aim(&mut engine, &[bolt], &[]);
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, my_wiz));
    assert_eq!(lk::life(&engine, P0), 20, "Bolt was countered by Wizard");
    assert!(in_graveyard(&engine, P1, lightning_bolt()).is_some());

    // Next turn the granted ability is gone.
    reach_their_main_phase(&mut engine, P0);
    assert!(granted_index(&engine, my_wiz).is_none());
}
