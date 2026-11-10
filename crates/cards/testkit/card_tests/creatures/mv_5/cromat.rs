//! Oracle scenarios for Cromat.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn cromat() -> CardIndex {
    card_index("376601b6-fe51-4e2d-8ec6-98f965d649a3")
}

#[test]
#[allow(clippy::too_many_lines)]
fn cromat_five_activated_abilities_destroy_blocker_grant_flying_regenerate_pump_and_topdeck() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                island(),
                island(),
                index::SWAMP,
                index::SWAMP,
                index::MOUNTAIN,
                index::MOUNTAIN,
                forest(),
                forest(),
                cromat(),
            ],
        )
        .battlefield(1, &[index::EARTH_ELEMENTAL, index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);

    let c = on_battlefield(&e, p0, cromat()).unwrap();
    let elemental = on_battlefield(&e, p1, index::EARTH_ELEMENTAL).unwrap();
    let bears = on_battlefield(&e, p1, index::GRIZZLY_BEARS).unwrap();

    tap_all_mana(&mut e, p0);
    // {U}{R}: Cromat gains flying until end of turn; {R}{W}: Cromat gets +1/+1 until end of turn;
    // {B}{G}: Regenerate Cromat.
    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: c,
            ability_index: 1,
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(
        e.state()
            .object(c)
            .unwrap()
            .characteristics()
            .keywords
            .contains(KeywordSet::FLYING)
    );

    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: c,
            ability_index: 3,
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, c), (6, 6));

    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: c,
            ability_index: 2,
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(c).unwrap().regeneration_shields, 1);

    // Move to p1's turn, untap p0's lands on p1's main phase for combat activations, then have
    // Earth Elemental attack and Cromat block it.
    reach_their_main_phase(&mut e, p1);
    for id in e.state().zones.list(ZoneLocation::Battlefield).clone() {
        if e.state().object(id).is_some_and(|o| o.controller == p0) {
            e.dev_state_mut(p0)
                .unwrap()
                .object_mut(id)
                .unwrap()
                .status
                .remove(Status::TAPPED);
        }
    }
    e.refresh_offer();

    pass_until(&mut e, |eng| {
        matches!(eng.pending(), Pending::ChooseAttackers { .. })
    });
    e.apply(
        p1,
        PlayerAction::DeclareAttackers {
            attackers: vec![(elemental, Defender::Player(p0))],
        },
    )
    .unwrap();
    pass_until(&mut e, |eng| {
        matches!(eng.pending(), Pending::ChooseBlockers { .. })
    });
    e.apply(
        p0,
        PlayerAction::DeclareBlockers {
            blockers: vec![(c, elemental)],
        },
    )
    .unwrap();

    // p1 passes priority in DeclareBlockers step so p0 can tap mana and activate {W}{B}.
    e.apply(p1, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut e, p0);
    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: c,
            ability_index: 0,
        },
    )
    .unwrap();
    if let Pending::ChooseTargets { options, .. } = e.pending().clone() {
        assert!(
            options.contains(&elemental) && !options.contains(&bears),
            "only the creature blocking or blocked by Cromat is a legal target"
        );
        e.apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elemental],
            },
        )
        .unwrap();
    }
    pass_until(&mut e, stack_is_empty);
    assert!(
        in_graveyard(&e, p1, index::EARTH_ELEMENTAL).is_some(),
        "Earth Elemental blocked by Cromat was destroyed by {{W}}{{B}}"
    );

    // Now p1 passes priority again and p0 activates {G}{U}: Put Cromat on top of its owner's library.
    e.apply(p1, PlayerAction::PassPriority).unwrap();
    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: c,
            ability_index: 4,
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(
        on_battlefield(&e, p0, cromat()).is_none(),
        "Cromat left the battlefield"
    );
    let top = *e
        .state()
        .zones
        .list(ZoneLocation::Library(p0))
        .last()
        .unwrap();
    assert_eq!(
        e.state().object(top).unwrap().card.unwrap().index,
        cromat(),
        "Cromat was put on top of its owner's library"
    );
}
