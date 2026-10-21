//! `cards/instants/mv_3/essence_vortex.rs`, played.
//!
//! Oracle:
//! - "Destroy target creature unless its controller pays life equal to its toughness. A creature destroyed this way can't be regenerated."

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn essence_vortex() -> CardIndex {
    card_index("ea38a211-a7b0-49fe-b5dc-7eca7610cccb")
}

/// P0 casts Essence Vortex at `target`; the question "pay life equal to its
/// toughness?" is answered `pay`. Returns the life asked for.
#[track_caller]
fn vortex_answering(engine: &mut Engine<RegistryLookup>, target: ObjectId, pay: bool) -> u32 {
    lk::float(
        engine,
        P0,
        &[
            (ManaColor::Colorless, 1),
            (ManaColor::Blue, 1),
            (ManaColor::Black, 1),
        ],
    );
    cast_with_floating(engine, P0, essence_vortex());
    lk::aim(engine, &[target], &[]);
    let mut asked = None;
    for _ in 0..20 {
        if stack_is_empty(engine) && matches!(engine.pending(), Pending::Priority { .. }) {
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::PayLife { amount },
                ..
            } => {
                assert_eq!(player, P1, "\"its controller\" is asked, not the caster");
                asked = Some(amount);
                engine.apply(player, PlayerAction::YesNo(pay)).unwrap();
            }
            other => panic!("unexpected pending {other:?}"),
        }
    }
    asked.expect("the creature's controller was offered the life payment")
}

#[test]
fn essence_vortex_destroys_target_unless_controller_pays_toughness_life() {
    let mut engine = lk::game(SEED, &[], &[index::GRIZZLY_BEARS], &[essence_vortex()], &[]);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    assert_eq!(
        vortex_answering(&mut engine, bears, false),
        2,
        "life equal to the Bears' toughness"
    );
    assert_eq!(lk::zone_of(&engine, bears), Zone::Graveyard);
    assert_eq!(lk::life(&engine, P1), 20, "declined, no life was paid");
}

/// The "unless" paid: the Bears survive and their controller is 2 life
/// poorer.
#[test]
fn essence_vortex_spares_the_creature_whose_controller_pays_its_toughness_in_life() {
    let mut engine = lk::game(SEED, &[], &[index::GRIZZLY_BEARS], &[essence_vortex()], &[]);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    assert_eq!(vortex_answering(&mut engine, bears, true), 2);
    assert_eq!(lk::zone_of(&engine, bears), Zone::Battlefield);
    assert_eq!(lk::life(&engine, P1), 18);
    assert_eq!(lk::life(&engine, P0), 20, "the caster pays nothing");
}

/// "A creature destroyed this way can't be regenerated" (CR 701.19c): Drudge
/// Skeletons with a regeneration shield up still go to the graveyard.
#[test]
fn essence_vortex_destroys_through_a_regeneration_shield() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[index::DRUDGE_SKELETONS],
        &[essence_vortex()],
        &[],
    );
    let skeletons = lk::obj(&engine, P1, index::DRUDGE_SKELETONS);

    lk::pass_once(&mut engine);
    lk::float(&mut engine, P1, &[(ManaColor::Black, 1)]);
    activate(&mut engine, P1, index::DRUDGE_SKELETONS, 0);
    lk::settle(&mut engine);
    assert_eq!(
        engine
            .state()
            .object(skeletons)
            .unwrap()
            .regeneration_shields,
        1,
        "the Skeletons hold a regeneration shield"
    );

    assert_eq!(vortex_answering(&mut engine, skeletons, false), 1);
    assert_eq!(
        lk::zone_of(&engine, skeletons),
        Zone::Graveyard,
        "the shield does not save it"
    );
}
