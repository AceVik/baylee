//! Canonical Oracle behavior, exercised through real spells and combat.
// oracle_id = "09b9e6fd-7a61-4ed4-a121-61b64fbf03f4"

#[allow(clippy::wildcard_imports)] // Shared canonical-card harness.
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn aim(e: &mut Engine<RegistryLookup>, seat: PlayerId, objects: &[ObjectId], players: &[PlayerId]) {
    let Pending::ChooseTargets { player, .. } = e.pending() else {
        panic!("target question: {:?}", e.pending())
    };
    assert_eq!(*player, seat);
    e.apply(
        seat,
        PlayerAction::ChooseTargets {
            objects: objects.to_vec(),
            players: players.to_vec(),
        },
    )
    .unwrap();
}

#[test]
fn batch57_shimian_redirects_only_the_target_attacker_and_pays_black_and_tap() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::SHIMIAN_NIGHT_STALKER, index::SWAMP])
        .battlefield(
            1,
            &[
                index::CRAW_WURM,
                index::GRIZZLY_BEARS,
                index::LLANOWAR_ELVES,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let stalker = on_battlefield(&e, P0, index::SHIMIAN_NIGHT_STALKER).unwrap();
    let wurm = on_battlefield(&e, P1, index::CRAW_WURM).unwrap();
    let bear = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    let elf = on_battlefield(&e, P1, index::LLANOWAR_ELVES).unwrap();
    assert!(
        !priority_offer(&e).abilities.contains(&(stalker, 0)),
        "nonattacking creatures are not legal targets"
    );
    kit::attack_on_their_turn(&mut e, P1, &[wurm, bear], P0);
    tap_all_mana(&mut e, P0);
    kit::activate_object(&mut e, P0, stalker, 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = e.pending()
    else {
        panic!("target attacking creature")
    };
    assert_eq!(*player, P0);
    assert!(options.contains(&wurm) && options.contains(&bear));
    assert!(!options.contains(&elf) && !options.contains(&stalker));
    aim(&mut e, P0, &[bear], &[]);
    pass_until(&mut e, stack_is_empty);
    assert!(is_tapped(&e, stalker));
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    e.apply(P0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(e.state().players[0].life, 14);
    assert_eq!(e.state().object(stalker).unwrap().damage, 2);
}

#[test]
fn batch57_shimian_redirects_all_noncombat_and_combat_damage_then_expires() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::SHIMIAN_NIGHT_STALKER, index::SWAMP])
        .battlefield(1, &[index::PRODIGAL_PYROMANCER, index::SERRA_S_BLESSING])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let stalker = on_battlefield(&e, P0, index::SHIMIAN_NIGHT_STALKER).unwrap();
    let pyro = on_battlefield(&e, P1, index::PRODIGAL_PYROMANCER).unwrap();
    kit::attack_on_their_turn(&mut e, P1, &[pyro], P0);
    assert!(
        !is_tapped(&e, pyro),
        "vigilance permits its noncombat ability during attack"
    );
    tap_all_mana(&mut e, P0);
    kit::activate_object(&mut e, P0, stalker, 0);
    aim(&mut e, P0, &[pyro], &[]);
    pass_until(&mut e, stack_is_empty);
    kit::activate_object(&mut e, P1, pyro, 0);
    aim(&mut e, P1, &[], &[P0]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(stalker).unwrap().damage, 1);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    e.apply(P0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(e.state().object(stalker).unwrap().damage, 2);
    assert_eq!(e.state().players[0].life, 20);
    reach_their_main_phase(&mut e, P0);
    reach_their_main_phase(&mut e, P1);
    kit::activate_object(&mut e, P1, pyro, 0);
    aim(&mut e, P1, &[], &[P0]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, 19);
    assert_eq!(e.state().object(stalker).unwrap().damage, 0);
}

#[test]
fn batch57_shimian_cannot_activate_without_black_mana_even_with_an_attacking_target() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::SHIMIAN_NIGHT_STALKER])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let stalker = on_battlefield(&e, P0, index::SHIMIAN_NIGHT_STALKER).unwrap();
    let bear = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    kit::attack_on_their_turn(&mut e, P1, &[bear], P0);
    assert!(!priority_offer(&e).abilities.contains(&(stalker, 0)));
    assert!(!is_tapped(&e, stalker));
}
