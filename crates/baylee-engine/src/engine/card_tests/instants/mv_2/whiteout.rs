//! Whiteout — `{1}{G}` Instant (Ice Age)
//!
//! All creatures lose flying until end of turn.
//! Sacrifice a snow land: Return Whiteout from your graveyard to your hand.

#[allow(clippy::wildcard_imports)]
use super::*;

fn whiteout() -> CardIndex {
    card_index("9e8de3e1-1159-460f-99b7-1043e5814597")
}

fn air_elemental() -> CardIndex {
    card_index("7744bae4-a8b7-44a5-9b4c-0048ad4cc448")
}

fn snow_covered_forest() -> CardIndex {
    card_index("5f0d3be8-e63e-4ade-ae58-6b0c14f2ce6d")
}

#[test]
fn whiteout_strips_flying_from_all_creatures_and_returns_from_graveyard_by_sacrificing_snow_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[whiteout(), snow_covered_forest()])
        .battlefield(0, &[forest(), forest(), air_elemental()])
        .battlefield(1, &[grizzly_bears(), air_elemental()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let our_flyer = on_battlefield(&engine, p0, air_elemental()).unwrap();
    let opp_flyer = on_battlefield(&engine, p1, air_elemental()).unwrap();
    let opp_bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let spell = in_hand(&engine, p0, whiteout()).unwrap();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Both Air Elementals lost flying until EOT.
    for flyer in [our_flyer, opp_flyer] {
        assert!(
            !engine
                .state()
                .object(flyer)
                .unwrap()
                .characteristics()
                .keywords
                .contains(KeywordSet::FLYING),
            "Whiteout removes flying from all creatures"
        );
    }

    // With only non-snow Forests on the battlefield, Whiteout's graveyard ability (index 1) is not offered.
    let gy_whiteout = in_graveyard(&engine, p0, whiteout()).unwrap();
    assert!(
        !ability_offered(&engine, gy_whiteout, 1),
        "non-snow Forest cannot pay Whiteout's Sacrifice a snow land cost"
    );

    // Play Snow-Covered Forest: now Whiteout's graveyard ability is offered!
    let snow_land = in_hand(&engine, p0, snow_covered_forest()).unwrap();
    engine
        .apply(p0, PlayerAction::PlayLand { card: snow_land })
        .unwrap();
    assert!(
        ability_offered(&engine, gy_whiteout, 1),
        "Snow-Covered Forest enables Whiteout's graveyard return ability"
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: gy_whiteout,
                ability_index: 1,
            },
        )
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![snow_land],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, snow_covered_forest()).is_some(),
        "Snow-Covered Forest was sacrificed"
    );
    assert!(
        in_hand(&engine, p0, whiteout()).is_some(),
        "Whiteout returned from graveyard to hand"
    );

    // Attack with our Air Elemental: ground Grizzly Bears can block it because it lost flying.
    let blocks = attack_and_collect_blocks(&mut engine, our_flyer, p1);
    assert!(
        blocks
            .iter()
            .any(|b| b.blocker == opp_bears && b.attackers.contains(&our_flyer)),
        "ground Grizzly Bears can block Air Elemental after Whiteout removes flying"
    );
}
