//! Gustha's Scepter — `{0}` Artifact (Alliances)
//!
//! (`Coverage::Partial`: exiling a card from your hand face down linked to this
//! artifact, returning a single chosen linked exiled card to your hand, and
//! triggering on losing control of this artifact are not supported; first
//! ability exiles the top card of your library face down linked to this
//! artifact, second ability returns all linked exiled cards to their owners'
//! hands, and leaves-the-battlefield trigger puts linked exiled cards into
//! their owners' graveyards.)

#[allow(clippy::wildcard_imports)]
use super::*;

fn gustha_s_scepter() -> CardIndex {
    card_index("120e9013-b07b-49d9-8b86-f6c26315581d")
}

fn disenchant() -> CardIndex {
    baylee_core::generated::index::DISENCHANT
}

#[test]
fn gustha_s_scepter_exiles_linked_returns_to_hand_and_puts_into_graveyard_on_ltb() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[disenchant()])
        .battlefield(0, &[gustha_s_scepter(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let scepter = on_battlefield(&engine, p0, gustha_s_scepter()).unwrap();
    let lib_before = engine.state().zones.list(ZoneLocation::Library(p0)).len();
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    // Activate ability 0 ({T}: ExileTopFaceDownLinked).
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: scepter,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p0)).len(),
        lib_before - 1
    );
    assert_eq!(engine.state().zones.list(ZoneLocation::Exile(p0)).len(), 1);

    // Untap on next turn and activate ability 1 ({T}: ReturnLinkedExile to hand).
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    let hand_mid = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: scepter,
                ability_index: 1,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().zones.list(ZoneLocation::Exile(p0)).len(), 0);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_mid + 1,
        "linked exiled card returned to hand (started with {hand_before})"
    );

    // Next turn, exile another card with ability 0, then destroy Gustha's Scepter with Disenchant -> LTB trigger puts linked exiled card into graveyard.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: scepter,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().zones.list(ZoneLocation::Exile(p0)).len(), 1);

    let gy_before = engine.state().zones.list(ZoneLocation::Graveyard(p0)).len();
    let dis = in_hand(&engine, p0, disenchant()).unwrap();
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: dis })
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![scepter],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().zones.list(ZoneLocation::Exile(p0)).len(), 0);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        gy_before + 3,
        "Disenchant + Gustha's Scepter + linked exiled card all in graveyard"
    );
}
