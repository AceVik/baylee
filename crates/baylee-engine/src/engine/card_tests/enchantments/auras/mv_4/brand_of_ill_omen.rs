//! `cards/enchantments/auras/mv_4/brand_of_ill_omen.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn brand_of_ill_omen() -> CardIndex {
    card_index("947bd180-ee95-4db4-b5d9-432428018eb8")
}

/// Brand of Ill Omen (947bd180-ee95-4db4-b5d9-432428018eb8) — {3}{R} Enchantment — Aura:
/// - "Enchant creature"
/// - "Cumulative upkeep {R}"
/// - "Enchanted creature's controller can't cast creature spells." (`Coverage::Partial`:
///   controller cast restriction not supported).
#[test]
fn brand_of_ill_omen_enchants_a_creature_and_has_cumulative_upkeep_red() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), mountain()])
        .battlefield(1, &[gray_ogre()])
        .hand(0, &[brand_of_ill_omen()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let ogre = on_battlefield(&engine, P1, gray_ogre()).expect("Gray Ogre seated");

    cast_from_hand(&mut engine, P0, brand_of_ill_omen());
    aim_at(&mut engine, P0, ogre);
    pass_until(&mut engine, stack_is_empty);

    let brand = on_battlefield(&engine, P0, brand_of_ill_omen()).expect("Brand seated");
    assert_eq!(
        engine.state().object(brand).unwrap().attached_to,
        Some(ogre)
    );

    // Advance to P0's next upkeep while the cumulative upkeep trigger is on the stack,
    // tap a Mountain for {R}, then resolve the trigger and pay {R}.
    reach_their_main_phase(&mut engine, P1);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::Upkeep && !stack_is_empty(e)
    });
    let mtn = on_battlefield(&engine, P0, mountain()).unwrap();
    engine
        .apply(P0, PlayerAction::ActivateManaAbility { source: mtn })
        .expect("tap Mountain for {R}");

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == P0),
    );
    assert_eq!(
        counters_on(&engine, brand, baylee_cards_dsl::counters::AGE),
        1
    );
    engine
        .apply(P0, PlayerAction::YesNo(true))
        .expect("pay cumulative upkeep {R}");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P0, brand_of_ill_omen()).is_some(),
        "paid cumulative upkeep {{R}}, so Brand stays on the battlefield"
    );
}
