//! `cards/enchantments/auras/mv_1/vanishing.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT};

fn vanishing() -> CardIndex {
    card_index("cd2791e8-0b61-49b8-8b72-ff2737df1049")
}

/// `Vanishing` ({U}, Enchantment — Aura, `Coverage::Implemented`) enchants a
/// creature and has `{U}{U}: Enchanted creature phases out.`
#[test]
fn vanishing_phases_out_enchanted_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[island(), island(), island(), GRIZZLY_BEARS, HILL_GIANT],
        )
        .hand(0, &[vanishing()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let other = on_battlefield(&engine, p0, HILL_GIANT).unwrap();

    cast_from_hand(&mut engine, p0, vanishing());
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, vanishing(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(bear)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );
    assert!(
        !engine
            .state()
            .object(other)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );
}

/// "It phases in before its controller untaps during their next untap step":
/// Vanishing on an opponent's creature phases it out, and it is back when
/// *that* player's next turn begins, not at the Aura's controller's untap.
#[test]
fn vanishing_on_an_opponents_creature_phases_it_in_at_its_controllers_untap() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .battlefield(1, &[GRIZZLY_BEARS])
        .hand(0, &[vanishing()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, p0, vanishing());
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);

    activate(&mut engine, p0, vanishing(), 1);
    pass_until(&mut engine, stack_is_empty);
    let phased_out = |engine: &Engine<RegistryLookup>| {
        engine
            .state()
            .object(bear)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    };
    assert!(phased_out(&engine), "the opponent's creature phased out");

    reach_their_main_phase(&mut engine, p1);
    assert!(
        !phased_out(&engine),
        "it phased in before its controller untapped"
    );
}
