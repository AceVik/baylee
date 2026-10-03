//! Independent real-card target identity review: CR 400.7, 608.2b,
//! 115.3, 115.7d–f, 701.14b, and 707.10c.

#[allow(clippy::wildcard_imports)] // Shared behavioral card-test vocabulary.
use super::*;
use baylee_core::generated::index;

const CASTER: PlayerId = PlayerId::new(0);
const DEFENDER: PlayerId = PlayerId::new(1);

fn priority(engine: &mut Engine<RegistryLookup>, player: PlayerId) {
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::Priority { player: holder, .. } if *holder == player),
    );
}

fn version(engine: &Engine<RegistryLookup>, object: ObjectId) -> u32 {
    engine.state().object(object).unwrap().version
}

fn blink_with_ephemerate(engine: &mut Engine<RegistryLookup>, player: PlayerId, object: ObjectId) {
    let before = version(engine, object);
    priority(engine, player);
    cast_from_hand(engine, player, ephemerate());
    aim(engine, vec![object], vec![]);
    pass_until(engine, |e| {
        version(e, object) != before && matches!(e.pending(), Pending::Priority { .. })
    });
    assert_eq!(
        engine.state().object(object).unwrap().zone,
        Zone::Battlefield
    );
}

fn fireball_at(
    engine: &mut Engine<RegistryLookup>,
    objects: Vec<ObjectId>,
    players: Vec<PlayerId>,
) {
    cast_from_hand(engine, CASTER, index::FIREBALL);
    engine.apply(CASTER, PlayerAction::ChooseNumber(5)).unwrap();
    aim(engine, objects, players);
}

fn fork_top(engine: &mut Engine<RegistryLookup>) {
    let original = top(engine);
    priority(engine, CASTER);
    cast_from_hand(engine, CASTER, fork());
    aim(engine, vec![original], vec![]);
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
}

fn reject_targets(engine: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>) {
    let hash = engine.fingerprint();
    let question = serde_json::to_value(engine.pending()).unwrap();
    let journal = engine.state().journal.len();
    assert!(
        engine
            .apply(
                CASTER,
                PlayerAction::ChooseTargets {
                    objects,
                    players: vec![]
                }
            )
            .is_err()
    );
    assert_eq!(
        engine.fingerprint(),
        hash,
        "rejection cannot alter staged or committed targets"
    );
    assert_eq!(serde_json::to_value(engine.pending()).unwrap(), question);
    assert_eq!(engine.state().journal.len(), journal);
}

#[test]
fn lightning_bolt_does_not_damage_the_elf_returned_by_real_ephemerate() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain()])
        .hand(0, &[lightning_bolt()])
        .battlefield(1, &[llanowar_elves(), plains()])
        .hand(1, &[ephemerate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, CASTER);
    let elf = on_battlefield(&engine, DEFENDER, llanowar_elves()).unwrap();
    cast_from_hand(&mut engine, CASTER, lightning_bolt());
    aim(&mut engine, vec![elf], vec![]);
    blink_with_ephemerate(&mut engine, DEFENDER, elf);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        on_battlefield(&engine, DEFENDER, llanowar_elves()),
        Some(elf)
    );
    assert_eq!(engine.state().object(elf).unwrap().damage, 0);
    assert!(in_graveyard(&engine, CASTER, lightning_bolt()).is_some());
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().players[1].life, 20);
}

#[test]
fn fireball_redistributes_all_five_damage_to_the_player_after_its_creature_target_blinks() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(); 7])
        .hand(0, &[index::FIREBALL])
        .battlefield(1, &[llanowar_elves(), plains()])
        .hand(1, &[ephemerate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, CASTER);
    let elf = on_battlefield(&engine, DEFENDER, llanowar_elves()).unwrap();
    fireball_at(&mut engine, vec![elf], vec![DEFENDER]);
    blink_with_ephemerate(&mut engine, DEFENDER, elf);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 15);
    assert_eq!(
        on_battlefield(&engine, DEFENDER, llanowar_elves()),
        Some(elf)
    );
    assert_eq!(engine.state().object(elf).unwrap().damage, 0);
    assert_eq!(engine.state().players[0].life, 20);
}

#[test]
fn khalni_ambush_neither_creature_fights_when_either_target_group_blinks() {
    for blink_first in [true, false] {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(
                0,
                &[fangren_hunter(), forest(), forest(), forest(), plains()],
            )
            .hand(0, &[khalni_ambush(), ephemerate()])
            .battlefield(1, &[wild_colos(), plains()])
            .hand(1, &[ephemerate()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, CASTER);
        let hunter = on_battlefield(&engine, CASTER, fangren_hunter()).unwrap();
        let colos = on_battlefield(&engine, DEFENDER, wild_colos()).unwrap();
        for land in all_on_battlefield(&engine, CASTER, forest()) {
            engine
                .apply(CASTER, PlayerAction::ActivateManaAbility { source: land })
                .unwrap();
        }
        cast_front_face(&mut engine, CASTER, khalni_ambush());
        aim(&mut engine, vec![hunter], vec![]);
        aim(&mut engine, vec![colos], vec![]);
        let (player, target) = if blink_first {
            (CASTER, hunter)
        } else {
            (DEFENDER, colos)
        };
        blink_with_ephemerate(&mut engine, player, target);
        pass_until(&mut engine, stack_is_empty);
        for object in [hunter, colos] {
            assert_eq!(
                engine.state().object(object).unwrap().zone,
                Zone::Battlefield
            );
            assert_eq!(
                engine.state().object(object).unwrap().damage,
                0,
                "neither creature fights when group {blink_first} is stale"
            );
        }
        assert_eq!(pt(&engine, hunter), (4, 4));
        assert_eq!(pt(&engine, colos), (2, 2));
        assert_eq!(engine.state().players[0].life, 20);
        assert_eq!(engine.state().players[1].life, 20);
    }
}

#[test]
fn fork_keeps_the_old_illegal_target_or_explicitly_selects_the_returned_creature() {
    for select_returned in [false, true] {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &[mountain(); 3])
            .hand(0, &[lightning_bolt(), fork()])
            .battlefield(1, &[index::WALL_OF_SWORDS, plains()])
            .hand(1, &[ephemerate()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, CASTER);
        let wall = on_battlefield(&engine, DEFENDER, index::WALL_OF_SWORDS).unwrap();
        cast_from_hand(&mut engine, CASTER, lightning_bolt());
        aim(&mut engine, vec![wall], vec![]);
        blink_with_ephemerate(&mut engine, DEFENDER, wall);
        fork_top(&mut engine);
        aim(
            &mut engine,
            if select_returned { vec![wall] } else { vec![] },
            vec![],
        );
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(engine.state().object(wall).unwrap().zone, Zone::Battlefield);
        assert_eq!(
            engine.state().object(wall).unwrap().damage,
            if select_returned { 3 } else { 0 }
        );
        assert_eq!(engine.state().players[0].life, 20);
        assert_eq!(engine.state().players[1].life, 20);
    }
}

#[test]
fn fork_fireball_duplicate_rejection_preserves_a_staged_swap_and_its_damage_outcome() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(); 9])
        .hand(0, &[index::FIREBALL, fork()])
        .battlefield(1, &[llanowar_elves(), index::WALL_OF_AIR, plains()])
        .hand(1, &[ephemerate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, CASTER);
    let elf = on_battlefield(&engine, DEFENDER, llanowar_elves()).unwrap();
    let wall = on_battlefield(&engine, DEFENDER, index::WALL_OF_AIR).unwrap();
    fireball_at(&mut engine, vec![elf, wall], vec![]);
    fork_top(&mut engine);
    aim(&mut engine, vec![wall], vec![]);
    reject_targets(&mut engine, vec![]); // Keeping the second wall would duplicate the first.
    reject_targets(&mut engine, vec![wall]);
    aim(&mut engine, vec![elf], vec![]);
    blink_with_ephemerate(&mut engine, DEFENDER, elf);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        on_battlefield(&engine, DEFENDER, llanowar_elves()),
        Some(elf)
    );
    assert_eq!(engine.state().object(elf).unwrap().damage, 0);
    assert!(
        in_graveyard(&engine, DEFENDER, index::WALL_OF_AIR).is_some(),
        "the copy's sole legal target takes five; the original has no legal targets left"
    );
    assert_eq!(engine.state().players[1].life, 20);
}

#[test]
fn fork_fireball_accepts_old_and_returned_incarnations_as_distinct_targets() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(); 9])
        .hand(0, &[index::FIREBALL, fork()])
        .battlefield(1, &[index::WALL_OF_AIR, plains()])
        .hand(1, &[ephemerate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, CASTER);
    let wall = on_battlefield(&engine, DEFENDER, index::WALL_OF_AIR).unwrap();
    fireball_at(&mut engine, vec![wall], vec![DEFENDER]);
    blink_with_ephemerate(&mut engine, DEFENDER, wall);
    fork_top(&mut engine);
    aim(&mut engine, vec![], vec![]); // Keep the old, now-illegal wall incarnation.
    aim(&mut engine, vec![wall], vec![]); // Replace the player target with the current wall.
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, DEFENDER, index::WALL_OF_AIR).is_some(),
        "the new wall is the copy's sole legal target and takes five"
    );
    assert_eq!(
        engine.state().players[1].life,
        15,
        "the original retains its player target, which takes all five"
    );
    assert_eq!(engine.state().players[0].life, 20);
}

#[test]
fn vantress_visions_keeps_divided_damage_on_old_and_current_incarnations_separately() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                island(),
                island(),
            ],
        )
        .hand(0, &[index::FURY, virtue_of_knowledge()])
        .battlefield(1, &[index::WALL_OF_AIR, index::WALL_OF_SWORDS, plains()])
        .hand(1, &[ephemerate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, CASTER);
    let first = on_battlefield(&engine, DEFENDER, index::WALL_OF_AIR).unwrap();
    let second = on_battlefield(&engine, DEFENDER, index::WALL_OF_SWORDS).unwrap();
    for land in all_on_battlefield(&engine, CASTER, mountain()) {
        engine
            .apply(CASTER, PlayerAction::ActivateManaAbility { source: land })
            .unwrap();
    }
    cast_front_face(&mut engine, CASTER, index::FURY);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim(&mut engine, vec![first, second], vec![]);
    engine.apply(CASTER, PlayerAction::ChooseNumber(3)).unwrap();
    let original = top(&engine);
    blink_with_ephemerate(&mut engine, DEFENDER, first);
    priority(&mut engine, CASTER);
    cast_from_hand(&mut engine, CASTER, virtue_of_knowledge());
    if let Pending::ChooseCastMode { options, .. } = engine.pending().clone() {
        let adventure = options
            .iter()
            .position(|option| matches!(option.kind, CastModeKind::Face(1)))
            .unwrap();
        engine
            .apply(CASTER, PlayerAction::ChooseMode(adventure))
            .unwrap();
    }
    aim(&mut engine, vec![original], vec![]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim(&mut engine, vec![], vec![]); // Keep the old first target's three damage.
    aim(&mut engine, vec![first], vec![]); // Move only the second target's one damage.
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(first).unwrap().damage,
        1,
        "the illegal old incarnation's three damage cannot land on the current incarnation"
    );
    assert_eq!(
        engine.state().object(second).unwrap().damage,
        1,
        "the original's unchanged second target still receives its fixed one damage"
    );
    assert_eq!(
        engine.state().object(first).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(
        engine.state().object(second).unwrap().zone,
        Zone::Battlefield
    );
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().players[1].life, 20);
}
