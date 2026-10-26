//! `cards/creatures/mv_2/avenger_en_dal.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Avenger en-Dal ({1}{W} 1/1 Creature — Human Spellshaper):
/// "{2}{W}, {T}, Discard a card: Exile target attacking creature. Its controller gains life equal to its toughness."
#[test]
fn avenger_en_dal_exiles_attacking_creature_and_its_controller_gains_life_equal_to_toughness() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[card_index("62009b63-ca6c-4fb0-8c9a-eccdec51cb18")])
        .hand(0, &[plains()])
        .battlefield(1, &[index::HILL_GIANT])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

    let giant = on_battlefield(&engine, P1, index::HILL_GIANT).unwrap();
    let discard = in_hand(&engine, P0, plains()).unwrap();

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P1),
    );
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(P0))],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );

    engine.dev_state_mut(P0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::White, 3);
    engine.refresh_offer();

    activate(
        &mut engine,
        P0,
        card_index("62009b63-ca6c-4fb0-8c9a-eccdec51cb18"),
        0,
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![discard],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P1, index::HILL_GIANT).is_none());
    assert_eq!(engine.state().zones.list(ZoneLocation::Exile(P1)).len(), 1);
    assert_eq!(engine.state().players[1].life, 23);
    assert!(in_graveyard(&engine, P0, plains()).is_some());
}
