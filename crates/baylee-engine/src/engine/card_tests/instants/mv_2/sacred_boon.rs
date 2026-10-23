//! `cards/instants/mv_2/sacred_boon.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn sacred_boon() -> CardIndex {
    card_index("32a8d49b-cdfb-4944-bfb5-afd704cc3658")
}

/// Sacred Boon (32a8d49b-cdfb-4944-bfb5-afd704cc3658) — {1}{W} Instant:
/// "Prevent the next 3 damage that would be dealt to target creature this turn."
/// (`Coverage::Partial` — delayed end-step +0/+1 counters not supported.)
#[test]
fn sacred_boon_prevents_the_next_three_damage_to_target_creature() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[plains(), plains(), mountain(), mountain(), grizzly_bears()],
        )
        .hand(0, &[sacred_boon(), lightning_bolt(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, grizzly_bears()).unwrap();

    // Cast Sacred Boon targeting Grizzly Bears.
    cast_from_hand_targeting(&mut engine, P0, sacred_boon(), bears);
    pass_until(&mut engine, stack_is_empty);

    // First Lightning Bolt (3 damage) is completely prevented by Sacred Boon's 3-damage shield.
    cast_from_hand_targeting(&mut engine, P0, lightning_bolt(), bears);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, P0, grizzly_bears()).is_some(),
        "Grizzly Bears survives the first Lightning Bolt because 3 damage was prevented"
    );
    assert_eq!(
        engine.state().object(bears).unwrap().damage,
        0,
        "all 3 damage was prevented"
    );

    // Second Lightning Bolt (3 damage) is unprevented and destroys Grizzly Bears.
    cast_from_hand_targeting(&mut engine, P0, lightning_bolt(), bears);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, P0, grizzly_bears()).is_none()
            && in_graveyard(&engine, P0, grizzly_bears()).is_some(),
        "with the 3-damage shield consumed, the second Lightning Bolt destroys Grizzly Bears"
    );
}

fn cast_from_hand_targeting(
    engine: &mut Engine<RegistryLookup>,
    player: PlayerId,
    card: CardIndex,
    target: ObjectId,
) {
    cast_from_hand(engine, player, card);
    engine
        .apply(
            player,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .unwrap();
}
