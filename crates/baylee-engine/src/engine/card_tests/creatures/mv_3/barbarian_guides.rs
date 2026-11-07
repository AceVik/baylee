//! `cards/creatures/mv_3/barbarian_guides.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::{index, subtypes::land};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn barbarian_guides() -> CardIndex {
    card_index("ef8c324a-582b-401e-add1-1985c5596908")
}

fn prepared(defender_lands: &[CardIndex]) -> Engine<RegistryLookup> {
    let mut defender = vec![llanowar_elves()];
    defender.extend_from_slice(defender_lands);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                barbarian_guides(),
                mountain(),
                mountain(),
                mountain(),
                gray_ogre(),
            ],
        )
        .battlefield(1, &defender)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

/// Activate at Gray Ogre and return the actual target menu with both objects.
fn activate_at_ogre(engine: &mut Engine<RegistryLookup>) -> (ObjectId, ObjectId, Vec<ObjectId>) {
    let guides = on_battlefield(engine, P0, barbarian_guides()).expect("Guides seated");
    let ogre = on_battlefield(engine, P0, gray_ogre()).expect("Gray Ogre seated");
    tap_all_mana_but(engine, P0, None);
    activate(engine, P0, barbarian_guides(), 0);
    let menu = aim_at(engine, P0, ogre);
    assert!(is_tapped(engine, guides), "the tap cost was paid");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the three red mana were spent, not left floating"
    );
    assert!(
        all_on_battlefield(engine, P0, mountain())
            .iter()
            .all(|land| is_tapped(engine, *land)),
        "all three Mountains paid the mana cost"
    );
    (guides, ogre, menu)
}

/// Resolve the real subtype question, including the boundaries of its offer.
fn resolve_landwalk(engine: &mut Engine<RegistryLookup>, chosen: baylee_core::ids::SubtypeId) {
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    let Pending::ChooseSubtype { player, options } = engine.pending().clone() else {
        unreachable!("the pass waited for the subtype question")
    };
    assert_eq!(player, P0);
    assert_eq!(
        options,
        baylee_core::generated::subtypes::ALL_LAND_TYPES
            .iter()
            .collect::<Vec<_>>(),
        "the Oracle text offers every land type, not only basic land types"
    );
    assert!(
        engine
            .apply(P1, PlayerAction::ChooseSubtype(chosen))
            .is_err(),
        "only the ability's controller chooses"
    );
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ChooseSubtype(baylee_core::generated::subtypes::creature::HUMAN),
            )
            .is_err(),
        "a creature type is not a land type"
    );
    engine
        .apply(P0, PlayerAction::ChooseSubtype(chosen))
        .expect("the chosen land type is offered");
}

/// Barbarian Guides (ef8c324a-582b-401e-add1-1985c5596908): {2}{R}, {T},
/// choose any land type, give a creature you control that snow landwalk
/// until end of turn, then return that exact creature at the next end step.
#[test]
fn barbarian_guides_targets_an_own_creature_pays_and_returns_it_at_next_end_step() {
    let mut engine = prepared(&[]);
    let their_elf = on_battlefield(&engine, P1, llanowar_elves()).expect("opponent Elf seated");
    let (guides, ogre, menu) = activate_at_ogre(&mut engine);
    assert_eq!(pt(&engine, guides), (1, 2));
    assert!(
        menu.contains(&ogre) && menu.contains(&guides),
        "creatures you control are legal targets"
    );
    assert!(
        !menu.contains(&their_elf),
        "opponent's creatures are excluded"
    );
    resolve_landwalk(&mut engine, land::MOUNTAIN);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, P0, gray_ogre()).is_some(),
        "still on the battlefield before the end step"
    );

    reach_their_main_phase(&mut engine, P1);
    assert!(
        on_battlefield(&engine, P0, gray_ogre()).is_none(),
        "returned from the battlefield at the beginning of the next end step"
    );
    assert!(
        in_hand(&engine, P0, gray_ogre()).is_some(),
        "in its owner's hand"
    );
}

/// Only a defending player's snow land with the chosen type prevents blocks.
#[test]
fn barbarian_guides_snow_mountainwalk_requires_snow_and_mountain_on_defenders_side() {
    for (defender_lands, may_block) in [
        (vec![index::SNOW_COVERED_MOUNTAIN], false),
        (vec![mountain()], true),
        (vec![index::SNOW_COVERED_FOREST], true),
    ] {
        let mut engine = prepared(&defender_lands);
        let (_, ogre, _) = activate_at_ogre(&mut engine);
        resolve_landwalk(&mut engine, land::MOUNTAIN);
        let elf = on_battlefield(&engine, P1, llanowar_elves()).expect("blocker seated");
        let offer = attack_and_collect_blocks(&mut engine, ogre, P1);
        assert_eq!(lk::may_block(&offer, elf, ogre), may_block);
        assert_eq!(
            engine
                .apply(
                    P1,
                    PlayerAction::DeclareBlockers {
                        blockers: vec![(elf, ogre)],
                    },
                )
                .is_ok(),
            may_block,
            "the declaration accepts exactly the pairings the offer contains"
        );
    }
}

/// Activation after one end step has begun delays the return to the next one;
/// the landwalk still expires in that turn's cleanup.
#[test]
fn barbarian_guides_landwalk_expires_but_delayed_return_waits_for_next_end_step() {
    let mut engine = prepared(&[index::SNOW_COVERED_MOUNTAIN]);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1
            && e.state().turn.step == Step::End
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P1)
    });
    engine
        .apply(P1, PlayerAction::PassPriority)
        .expect("the active player yields priority in their own end step");
    let (_, ogre, _) = activate_at_ogre(&mut engine);
    resolve_landwalk(&mut engine, land::MOUNTAIN);

    reach_main_phase(&mut engine, P0);
    assert!(
        on_battlefield(&engine, P0, gray_ogre()).is_some(),
        "the following end step has not happened yet"
    );
    let elf = on_battlefield(&engine, P1, llanowar_elves()).expect("blocker seated");
    let offer = attack_and_collect_blocks(&mut engine, ogre, P1);
    assert!(
        lk::may_block(&offer, elf, ogre),
        "snow mountainwalk wore off at the previous cleanup"
    );
    lk::block(&mut engine, &[(elf, ogre)]);

    reach_their_main_phase(&mut engine, P1);
    assert!(on_battlefield(&engine, P0, gray_ogre()).is_none());
    assert!(
        in_hand(&engine, P0, gray_ogre()).is_some(),
        "the delayed trigger still returns the original target"
    );
}

/// "A land type" includes nonbasic types; the snow requirement still applies.
#[test]
fn barbarian_guides_grants_snow_desertwalk_only_against_a_snow_desert() {
    for snow in [false, true] {
        let mut engine = prepared(&[desert()]);
        let desert = on_battlefield(&engine, P1, desert()).expect("Desert seated");
        if snow {
            let state = engine.dev_state_mut(P0).expect("test state access");
            let chars = state.object_mut(desert).expect("Desert object").base_mut();
            chars.supertypes = chars
                .supertypes
                .union(baylee_core::types::SupertypeSet::SNOW);
            state.invalidate_projections();
        }
        let (_, ogre, _) = activate_at_ogre(&mut engine);
        resolve_landwalk(&mut engine, land::DESERT);
        let elf = on_battlefield(&engine, P1, llanowar_elves()).expect("blocker seated");
        let offer = attack_and_collect_blocks(&mut engine, ogre, P1);
        assert_eq!(lk::may_block(&offer, elf, ogre), !snow);
        assert_eq!(
            engine
                .apply(
                    P1,
                    PlayerAction::DeclareBlockers {
                        blockers: vec![(elf, ogre)],
                    },
                )
                .is_ok(),
            !snow
        );
    }
}
