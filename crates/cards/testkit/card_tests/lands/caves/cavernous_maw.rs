//! `cards/lands/caves/cavernous_maw.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Cavernous Maw prints `{{T}}: Add {{C}}` and `{{2}}: This land becomes a 3/3 Elemental creature until end of turn. It's still a Cave land.`
/// Paying `{{2}}` to activate the animation turns the land into an untapped 3/3 Elemental creature while retaining its land type.
#[test]
fn cavernous_maw_animates_into_a_three_three_elemental_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                baylee_core::generated::index::HIDDEN_COURTYARD,
                baylee_core::generated::index::HIDDEN_CATARACT,
                baylee_core::generated::index::HIDDEN_VOLCANO,
            ],
        )
        .hand(0, &[cavernous_maw()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let maw = play_land(&mut engine, p0, cavernous_maw());
    assert!(!entered_tapped(&engine, maw));
    assert!(!types(&engine, maw).contains(TypeSet::CREATURE));

    tap_all_mana_but(&mut engine, p0, Some(cavernous_maw()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        2
    );

    activate(&mut engine, p0, cavernous_maw(), 1);
    pass_until(&mut engine, stack_is_empty);

    let t = types(&engine, maw);
    assert!(t.contains(TypeSet::CREATURE));
    assert!(t.contains(TypeSet::LAND));
    assert_eq!(pt(&engine, maw), (3, 3));
    assert!(!is_tapped(&engine, maw));
    let subtypes = engine
        .state()
        .object(maw)
        .unwrap()
        .characteristics()
        .subtypes;
    assert!(
        subtypes.contains(baylee_core::generated::subtypes::creature::ELEMENTAL),
        "\"becomes a 3/3 Elemental creature\": it is an Elemental"
    );
    assert!(
        subtypes.contains(baylee_core::generated::subtypes::land::CAVE),
        "\"It's still a Cave land\": it keeps its Cave subtype"
    );
}

#[test]
fn maw_combines_only_other_controlled_caves_with_its_owners_graveyard_caves() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for count in 0..=3 {
        let mut e = Duel::new(SEED, index::HIDDEN_COURTYARD)
            .battlefield(0, &[cavernous_maw(), index::HIDDEN_CATARACT])
            .battlefield(
                1,
                &[
                    index::HIDDEN_VOLCANO,
                    index::HIDDEN_NECROPOLIS,
                    index::HIDDEN_NURSERY,
                ],
            )
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        seed_graveyard(&mut e, p0, count);
        seed_graveyard(&mut e, p1, 4);
        let maw = on_battlefield(&e, p0, cavernous_maw()).unwrap();
        e.dev_state_mut(p0).unwrap().players[0]
            .mana_pool
            .add(ManaColor::Colorless, 2);
        e.refresh_offer();
        let Pending::Priority { legal, .. } = e.pending() else {
            panic!("priority")
        };
        assert_eq!(legal.abilities.contains(&(maw, 1)), count >= 2);
        if count >= 2 {
            activate(&mut e, p0, cavernous_maw(), 1);
            pass_until(&mut e, stack_is_empty);
            assert_eq!(pt(&e, maw), (3, 3));
        } else {
            let hash = e.snapshot_hash();
            assert!(
                e.apply(
                    p0,
                    PlayerAction::ActivateAbility {
                        source: maw,
                        ability_index: 1
                    }
                )
                .is_err()
            );
            assert_eq!(hash, e.snapshot_hash());
        }
    }
}
