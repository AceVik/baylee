//! `cards/enchantments/auras/mv_3/krovikan_fetish.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::Step;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn krovikan_fetish() -> CardIndex {
    card_index("20c7630d-a97f-4f09-9ec5-b7a1739107de")
}

/// Krovikan Fetish (20c7630d-a97f-4f09-9ec5-b7a1739107de) — {2}{B} Enchantment — Aura:
/// - "Enchant creature"
/// - "When this Aura enters, draw a card at the beginning of the next turn's upkeep."
/// - "Enchanted creature gets +1/+1."
#[test]
fn krovikan_fetish_gives_enchanted_creature_plus_one_plus_one_and_draws_on_next_turns_upkeep() {
    let mut engine = Duel::new(SEED, index::SWAMP)
        .battlefield(
            0,
            &[
                index::SWAMP,
                index::SWAMP,
                index::SWAMP,
                index::GRIZZLY_BEARS,
                llanowar_elves(),
            ],
        )
        .hand(0, &[krovikan_fetish()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).expect("Bears seated");
    let elf = on_battlefield(&engine, P0, llanowar_elves()).expect("Elf seated");

    tap_all_mana_but(&mut engine, P0, Some(llanowar_elves()));
    cast_from_hand(&mut engine, P0, krovikan_fetish());
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, bears),
        (3, 3),
        "enchanted Grizzly Bears gets +1/+1"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "unenchanted Llanowar Elves is untouched"
    );

    let hand_after_cast = engine.state().zones.list(ZoneLocation::Hand(P0)).len();

    // At the beginning of the next turn's (P1's) upkeep, P0 draws a card.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_after_cast + 1,
        "ETB delayed trigger draws a card at the beginning of the next turn's upkeep"
    );
}
