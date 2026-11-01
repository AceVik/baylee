//! `cards/creatures/mv_1/benalish_missionary.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;
use baylee_core::generated::index::{GRIZZLY_BEARS, HILL_GIANT};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn benalish_missionary() -> CardIndex {
    card_index("84fdcfd3-2b22-4570-af34-7e3f55f97466")
}

/// P0 has a Hill Giant and a Llanowar Elves; P1 has two Plains, a Bears and a
/// Missionary that has been under their control for a whole turn. It is P0's
/// second turn, at the question of who attacks.
fn board() -> Engine<RegistryLookup> {
    let mut engine = lk::game(
        SEED,
        &[HILL_GIANT, llanowar_elves()],
        &[plains(), plains(), GRIZZLY_BEARS, benalish_missionary()],
        &[],
        &[],
    );
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    engine
}

/// `Benalish Missionary` ({W}, 1/1 Creature — Human Cleric,
/// `Coverage::Partial`): "{1}{W}, {T}: Prevent all combat damage that would be
/// dealt by target blocked creature this turn." The blocked Giant deals none to
/// the Bears that blocks it (while the Bears still hurts the Giant) and the
/// unblocked Elves is not on the target menu and hits for its 1.
#[test]
fn benalish_missionary_prevents_combat_damage_by_target_blocked_creature_only() {
    let mut engine = board();
    let giant = lk::obj(&engine, P0, HILL_GIANT);
    let elf = lk::obj(&engine, P0, llanowar_elves());
    let bears = lk::obj(&engine, P1, GRIZZLY_BEARS);
    let missionary = lk::obj(&engine, P1, benalish_missionary());

    lk::attack(&mut engine, &[giant, elf]);
    lk::block(&mut engine, &[(bears, giant)]);
    lk::to_step(&mut engine, P1, Step::DeclareBlockers);

    tap_all_mana(&mut engine, P1);
    assert!(lk::offered(&engine, benalish_missionary(), P1, 0));
    activate(&mut engine, P1, benalish_missionary(), 0);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("a target was expected, got {:?}", engine.pending());
    };
    assert!(options.contains(&giant), "the blocked attacker is a target");
    assert!(!options.contains(&elf), "an unblocked attacker is not");
    assert!(
        !options.contains(&bears),
        "a creature that is not attacking is not"
    );
    lk::aim(&mut engine, &[giant], &[]);
    lk::settle(&mut engine);
    assert!(lk::tapped(&engine, missionary), "{{T}} was paid");

    lk::through_combat(&mut engine);

    assert_eq!(
        lk::damage_on(&engine, bears),
        0,
        "the Giant's damage was prevented"
    );
    assert_eq!(lk::zone_of(&engine, bears), Zone::Battlefield);
    assert_eq!(
        lk::damage_on(&engine, giant),
        2,
        "the Bears' own damage is not prevented"
    );
    assert_eq!(
        lk::life(&engine, P1),
        19,
        "the unblocked Elves still connected"
    );
}

/// Without the activation the Giant kills the Bears.
#[test]
fn benalish_missionary_leaves_combat_damage_alone_unless_activated() {
    let mut engine = board();
    let giant = lk::obj(&engine, P0, HILL_GIANT);
    let bears = lk::obj(&engine, P1, GRIZZLY_BEARS);

    lk::attack(&mut engine, &[giant]);
    lk::block(&mut engine, &[(bears, giant)]);
    lk::through_combat(&mut engine);

    assert!(in_graveyard(&engine, P1, GRIZZLY_BEARS).is_some());
}

/// "Target blocked creature": before blockers are declared no attacker is
/// blocked yet, so the ability cannot be activated at all in the declare
/// attackers step.
#[test]
fn benalish_missionary_cannot_be_activated_before_blockers_are_declared() {
    let mut engine = board();
    let giant = lk::obj(&engine, P0, HILL_GIANT);
    let missionary = lk::obj(&engine, P1, benalish_missionary());
    let able = lk::attackers_offered(&mut engine);
    assert!(able.contains(&giant));
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(giant, Defender::Player(P1))],
            },
        )
        .unwrap();
    lk::to_step(&mut engine, P1, Step::DeclareAttackers);

    tap_all_mana(&mut engine, P1);
    assert!(
        !lk::offered(&engine, benalish_missionary(), P1, 0),
        "not offered before the blocks"
    );
    assert!(
        engine
            .apply(
                P1,
                PlayerAction::ActivateAbility {
                    source: missionary,
                    ability_index: 0,
                },
            )
            .is_err(),
        "and refused if asked"
    );
}
