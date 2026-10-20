//! `cards/creatures/mv_3/adarkar_unicorn.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn adarkar_unicorn() -> CardIndex {
    card_index("287cd5e8-51e2-42a4-a139-c112ff89706d")
}

fn arnjlot_s_ascent() -> CardIndex {
    card_index("f2cedb38-12b0-4489-a62b-9b6a3776d695")
}

/// Adarkar Unicorn (287cd5e8-51e2-42a4-a139-c112ff89706d) — {1}{W}{W} 2/2
/// Creature — Unicorn:
/// - "{T}: Add {U} or {C}{U}. Spend this mana only to pay cumulative upkeep costs."
#[test]
fn adarkar_unicorn_adds_restricted_mana_for_cumulative_upkeep_and_not_normal_spells() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[adarkar_unicorn()])
        .hand(0, &[unsummon()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let unicorn = on_battlefield(&engine, P0, adarkar_unicorn()).expect("Unicorn seated");
    assert_eq!(pt(&engine, unicorn), (2, 2));

    // Activate second mana ability: {T}: Add {C}{U} (restricted to CumulativeUpkeep)
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: unicorn,
                ability_index: 1,
            },
        )
        .expect("tap Adarkar Unicorn for {C}{U}");
    assert!(is_tapped(&engine, unicorn));
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    assert_eq!(
        engine.state().players[0].mana_pool.restricted().len(),
        2,
        "both mana points carry the CumulativeUpkeep restriction"
    );

    // Unsummon costs {U}, which cannot be paid with cumulative-upkeep-only mana.
    let offer = priority_offer(&engine);
    let unsummon_id = in_hand(&engine, P0, unsummon()).unwrap();
    assert!(
        !offer.castable.contains(&unsummon_id),
        "cumulative-upkeep mana cannot cast Unsummon"
    );
}

#[test]
fn adarkar_unicorn_mana_pays_cumulative_upkeep_cost() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[adarkar_unicorn(), island(), island(), island()])
        .hand(0, &[arnjlot_s_ascent()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let unicorn = on_battlefield(&engine, P0, adarkar_unicorn()).expect("Unicorn seated");
    tap_all_mana_but(&mut engine, P0, Some(adarkar_unicorn()));
    cast_with_floating(&mut engine, P0, arnjlot_s_ascent());
    pass_until(&mut engine, stack_is_empty);

    // Advance to P1's main phase, then to P0's next upkeep when the cumulative
    // upkeep trigger is on the stack.
    reach_their_main_phase(&mut engine, P1);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::Upkeep && !stack_is_empty(e)
    });

    // Tap Adarkar Unicorn for {U} (ability 0) before the trigger resolves.
    engine
        .apply(
            P0,
            PlayerAction::ActivateAbility {
                source: unicorn,
                ability_index: 0,
            },
        )
        .expect("tap Adarkar Unicorn for {U}");

    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == P0),
    );
    engine
        .apply(P0, PlayerAction::YesNo(true))
        .expect("pay cumulative upkeep {U} with Adarkar Unicorn's restricted mana");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, P0, arnjlot_s_ascent()).is_some(),
        "Arnjlot's Ascent stayed on the battlefield after paying cumulative upkeep with Adarkar Unicorn"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
}
