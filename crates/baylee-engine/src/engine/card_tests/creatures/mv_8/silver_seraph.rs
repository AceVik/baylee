//! `cards/creatures/mv_8/silver_seraph.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn silver_seraph_pumps_other_creatures_you_control_by_two_two_at_threshold() {
    let seraph = card_index("bcc7e32a-9cf9-4fdf-a0a4-e90ae42f1353");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));

    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                swamp(),
                savannah_lions(),
            ],
        )
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[seraph, dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 6);
    for p in all_on_battlefield(&engine, p0, plains()) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: p })
            .unwrap();
    }
    let card = in_hand(&engine, p0, seraph).unwrap();
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let angel = on_battlefield(&engine, p0, seraph).expect("Silver Seraph on battlefield");
    let lions = on_battlefield(&engine, p0, savannah_lions()).expect("Savannah Lions");
    let opp_bears = on_battlefield(&engine, p1, grizzly_bears()).expect("Opponent Grizzly Bears");

    assert!(keywords(&engine, angel).contains(KeywordSet::FLYING));
    assert_eq!(pt(&engine, angel), (6, 6));
    assert_eq!(pt(&engine, lions), (2, 1));
    assert_eq!(pt(&engine, opp_bears), (2, 2));

    let s = all_on_battlefield(&engine, p0, swamp())[0];
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: s })
        .unwrap();
    let dr = in_hand(&engine, p0, dark_ritual()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: dr })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, angel),
        (6, 6),
        "Silver Seraph only buffs other creatures you control"
    );
    assert_eq!(pt(&engine, lions), (4, 3));
    assert_eq!(pt(&engine, opp_bears), (2, 2));
}
