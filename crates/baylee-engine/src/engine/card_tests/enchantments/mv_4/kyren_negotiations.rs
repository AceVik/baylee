//! `cards/enchantments/mv_4/kyren_negotiations.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "50090188-651d-4402-968f-3f6bf9276029"
fn kyren_negotiations() -> CardIndex {
    card_index("50090188-651d-4402-968f-3f6bf9276029")
}

fn activate_targeting_player_and_tapping(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    target: PlayerId,
    creature: ObjectId,
) -> Vec<ObjectId> {
    activate(engine, seat, kyren_negotiations(), 0);
    let mut cost_options = Vec::new();
    for _ in 0..4 {
        match engine.pending().clone() {
            Pending::ChooseTargets { .. } => {
                engine
                    .apply(
                        seat,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![target],
                        },
                    )
                    .unwrap();
            }
            Pending::ChoosePlayer { .. } => {
                engine
                    .apply(seat, PlayerAction::ChoosePlayer(target))
                    .unwrap();
            }
            Pending::ChooseCards { options, .. } => {
                cost_options = options;
                engine
                    .apply(
                        seat,
                        PlayerAction::ChooseObjects {
                            objects: vec![creature],
                        },
                    )
                    .unwrap();
                break;
            }
            other => panic!("unexpected pending prompt: {other:?}"),
        }
    }
    pass_until(engine, stack_is_empty);
    cost_options
}

/// `Kyren Negotiations` (`Coverage::Generated`):
/// "Tap an untapped creature you control: This enchantment deals 1 damage to
/// target player or planeswalker."
///
/// Verifies that `Kyren Negotiations` can tap any untapped creature its
/// controller controls (including a summoning-sick creature, since the cost does
/// not use the `{T}` symbol) to deal 1 damage to a target player, and cannot
/// tap already-tapped creatures or creatures controlled by an opponent.
#[test]
fn kyren_negotiations_taps_untapped_controlled_creatures_to_damage_target_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(9502, mountain())
        .battlefield(0, &[kyren_negotiations(), forest(), grizzly_bears()])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0));

    let negotiations =
        on_battlefield(&engine, p0, kyren_negotiations()).expect("P0 Kyren Negotiations");
    let bear = on_battlefield(&engine, p0, grizzly_bears()).expect("P0 Grizzly Bears");
    let their_bear = on_battlefield(&engine, p1, grizzly_bears()).expect("P1 Grizzly Bears");

    // Cast Llanowar Elves this turn so it has summoning sickness.
    cast_from_hand(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    let sick_elf = on_battlefield(&engine, p0, llanowar_elves()).expect("P0 summoning-sick Elf");

    // Activate Kyren Negotiations using Grizzly Bears to deal 1 damage to P1.
    let options = activate_targeting_player_and_tapping(&mut engine, p0, p1, bear);
    assert!(
        options.contains(&bear) && options.contains(&sick_elf),
        "both ready and summoning-sick untapped creatures controlled by P0 are legal TapOther choices"
    );
    assert!(
        !options.contains(&their_bear),
        "opponent's creature cannot be tapped for Kyren Negotiations"
    );
    assert!(is_tapped(&engine, bear));
    assert_eq!(life_of(&engine, p1), 19);

    // Activate a second time tapping the summoning-sick Llanowar Elves.
    let options = activate_targeting_player_and_tapping(&mut engine, p0, p1, sick_elf);
    assert_eq!(
        options,
        vec![sick_elf],
        "only the remaining untapped creature controlled by P0 is offered"
    );
    assert!(is_tapped(&engine, sick_elf));
    assert_eq!(life_of(&engine, p1), 18);

    // With no untapped creatures remaining under P0's control, the ability is no longer legal.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected Priority, got {:?}", engine.pending());
    };
    assert!(
        !legal.abilities.contains(&(negotiations, 0)),
        "Kyren Negotiations cannot be activated when P0 controls no untapped creatures"
    );
}
