//! `cards/enchantments/mv_3/story_circle.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn story_circle() -> CardIndex {
    card_index("7071aee8-b5ca-4be5-9ba0-2df7e3af303b")
}

/// As it enters, choose a color; `{W}` chooses a source of that color and
/// prevents the next time it would deal damage to you this turn (filtering
/// out other colors, sparing only the next hit to you).
#[test]
fn story_circle_chooses_color_on_etb_and_prevents_next_damage_from_chosen_source() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = lk::game(
        SEED,
        &[grizzly_bears()],
        &[orcish_artillery(), grizzly_bears()],
        &[story_circle()],
        &[lightning_bolt()],
    );
    let artillery = lk::obj(&e, p1, orcish_artillery());
    let opp_bear = lk::obj(&e, p1, grizzly_bears());

    lk::float(&mut e, p0, &[(ManaColor::White, 5)]);
    cast_from_hand(&mut e, p0, story_circle());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    e.apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();
    lk::settle(&mut e);

    // P1 fires Orcish Artillery at P0.
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    e.apply(
        p1,
        PlayerAction::ActivateAbility {
            source: artillery,
            ability_index: 0,
        },
    )
    .unwrap();
    lk::aim(&mut e, &[], &[p0]);

    // P0 activates Story Circle: only Red sources (Artillery, not Green Grizzly Bears) are offered.
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    activate(&mut e, p0, story_circle(), 0);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseDamageSource { .. })
    });
    let Pending::ChooseDamageSource {
        options, choice, ..
    } = e.pending().clone()
    else {
        unreachable!()
    };
    let artillery_ref = baylee_core::ids::DamageSourceRef {
        object: artillery,
        version: e.state().object(artillery).unwrap().version,
    };
    let bear_ref = baylee_core::ids::DamageSourceRef {
        object: opp_bear,
        version: e.state().object(opp_bear).unwrap().version,
    };
    assert!(options.contains(&artillery_ref));
    assert!(!options.contains(&bear_ref));
    e.apply(
        p0,
        PlayerAction::ChooseDamageSource {
            choice,
            source: artillery_ref,
        },
    )
    .unwrap();
    lk::settle(&mut e);
    assert_eq!(lk::life(&e, p0), 20);
    assert_eq!(lk::life(&e, p1), 17);

    // Unshielded Red Bolt still hits P0.
    lk::float(&mut e, p1, &[(ManaColor::Red, 1)]);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    let bolt = e.state().zones.list(ZoneLocation::Hand(p1))[0];
    e.apply(p1, PlayerAction::CastSpell { card: bolt }).unwrap();
    lk::aim(&mut e, &[], &[p0]);
    lk::settle(&mut e);
    assert_eq!(lk::life(&e, p0), 17);
}
