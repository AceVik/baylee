//! Faerie Noble — `{2}{G}` 1/2 Creature — Faerie Noble (Homelands)
//!
//! Flying
//! Other Faerie creatures you control get +0/+1.
//! `{T}`: Other Faerie creatures you control get +1/+0 until end of turn.

#[allow(clippy::wildcard_imports)]
use super::*;

fn faerie_noble() -> CardIndex {
    card_index("6172e5b9-2a01-429c-a140-e9b22fbab56e")
}

fn scryb_sprites() -> CardIndex {
    card_index("a1f20695-6f08-4d5c-9fba-b0018bee298e")
}

#[test]
fn faerie_noble_gives_other_faeries_you_control_plus_zero_plus_one_and_taps_for_plus_one_plus_zero()
{
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[faerie_noble(), scryb_sprites(), grizzly_bears()])
        .battlefield(1, &[scryb_sprites()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let noble = on_battlefield(&engine, p0, faerie_noble()).unwrap();
    let ally_faerie = on_battlefield(&engine, p0, scryb_sprites()).unwrap();
    let ally_bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let opp_faerie = on_battlefield(&engine, p1, scryb_sprites()).unwrap();

    assert_eq!(
        pt(&engine, noble),
        (1, 2),
        "Faerie Noble does not buff itself"
    );
    assert!(keywords(&engine, noble).contains(KeywordSet::FLYING));
    assert_eq!(
        pt(&engine, ally_faerie),
        (1, 2),
        "other Faerie you control gets +0/+1 (1/1 -> 1/2)"
    );
    assert_eq!(pt(&engine, ally_bears), (2, 2), "non-Faerie is unaffected");
    assert_eq!(
        pt(&engine, opp_faerie),
        (1, 1),
        "opponent's Faerie is unaffected"
    );

    // Activate {T}: Other Faerie creatures you control get +1/+0 until end of turn.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: noble,
                ability_index: 1,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, noble), (1, 2));
    assert_eq!(
        pt(&engine, ally_faerie),
        (2, 2),
        "other Faerie you control gets +1/+0 until end of turn (1/2 -> 2/2)"
    );
    assert_eq!(pt(&engine, opp_faerie), (1, 1));
}
