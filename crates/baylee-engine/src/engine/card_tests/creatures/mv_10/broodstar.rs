//! Broodstar — `{8}{U}{U}` */* Creature — Beast (MRD).
//! Oracle: "Affinity for artifacts. Flying. Broodstar's power and toughness are each equal to the number of artifacts you control."

use super::*;

fn broodstar() -> CardIndex {
    card_index("428cc1cb-a491-4cc4-9436-28ee8acc37d6")
}

#[test]
fn broodstar_reduces_cost_with_affinity_and_sets_pt_to_artifacts_you_control() {
    let p0 = PlayerId::new(0);
    let star = broodstar();
    let ring = sol_ring();
    // 6 Sol Rings reduce {8}{U}{U} by {6} to {2}{U}{U}, paid with 4 Islands.
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                ring,
                ring,
                ring,
                ring,
                ring,
                ring,
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .hand(0, &[star])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let islands = all_on_battlefield(&engine, p0, island());
    tap_mana_where(&mut engine, p0, |id| islands.contains(&id));
    cast_with_floating(&mut engine, p0, star);
    pass_until(&mut engine, stack_is_empty);

    let star_id = on_battlefield(&engine, p0, star).unwrap();
    assert_eq!(pt(&engine, star_id), (6, 6));
    assert!(keywords(&engine, star_id).contains(KeywordSet::FLYING));
}
