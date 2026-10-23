//! `cards/enchantments/auras/mv_1/cloak_of_invisibility.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT, WALL_OF_STONE};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn cloak_of_invisibility() -> CardIndex {
    card_index("68b06917-7f83-47d4-a466-4e6698cae56d")
}

/// Cloak of Invisibility (68b06917-7f83-47d4-a466-4e6698cae56d) — {U} Aura:
/// "Enchant creature. Enchanted creature has phasing and can't be blocked
/// except by Walls."
#[test]
fn cloak_of_invisibility_grants_phasing_and_restricts_blockers_to_walls() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), GRIZZLY_BEARS])
        .battlefield(1, &[HILL_GIANT, WALL_OF_STONE])
        .hand(0, &[cloak_of_invisibility()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, GRIZZLY_BEARS).unwrap();
    let giant = on_battlefield(&engine, P1, HILL_GIANT).unwrap();
    let wall = on_battlefield(&engine, P1, WALL_OF_STONE).unwrap();

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, cloak_of_invisibility());
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, bears).contains(KeywordSet::PHASING));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });

    // Non-Wall Hill Giant cannot block; Wall of Stone can block.
    assert!(
        engine
            .apply(
                P1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(giant, bears)],
                },
            )
            .is_err()
    );
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(wall, bears)],
            },
        )
        .unwrap();
}

/// "phasing": the enchanted creature phases out at its controller's untap
/// step, not at the opponent's, and back in at the next one.
#[test]
fn cloak_of_invisibility_phases_the_creature_out_at_its_controllers_untap() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), GRIZZLY_BEARS])
        .hand(0, &[cloak_of_invisibility()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, GRIZZLY_BEARS).unwrap();
    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, cloak_of_invisibility());
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    let phased_out = |e: &Engine<RegistryLookup>| {
        e.state()
            .object(bears)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    };
    assert!(!phased_out(&engine), "just enchanted");

    reach_their_main_phase(&mut engine, P1);
    assert!(
        !phased_out(&engine),
        "the opponent's untap step is not its controller's"
    );

    reach_their_main_phase(&mut engine, P0);
    assert!(phased_out(&engine), "its controller's next untap step");

    reach_their_main_phase(&mut engine, P1);
    assert!(phased_out(&engine), "still out in the opponent's turn");

    reach_their_main_phase(&mut engine, P0);
    assert!(
        !phased_out(&engine),
        "phased back in at the following untap step"
    );
}
