//! September 28 reports: Mind Twist paid its X cost but had no effect.
use super::testkit::*;
use super::*;
use baylee_core::ids::CardIndex;

fn twist() -> CardIndex {
    card_index("78f9c223-9982-4282-a496-a6f892f0a5bf")
}
fn swamp() -> CardIndex {
    card_index("56719f6a-1a6c-4c0a-8d21-18f7d7350b68")
}

fn cast(seed: u64, x: u32, victim: PlayerId) -> Engine<RegistryLookup> {
    let me = PlayerId::new(0);
    let mut engine = Duel::table(seed, basic_forest(), 3)
        .battlefield(0, &[swamp(); 12])
        .hand(0, &[twist()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, me));
    cast_from_hand(&mut engine, me, twist());
    assert!(matches!(engine.pending(), Pending::ChooseNumber { .. }));
    engine.apply(me, PlayerAction::ChooseNumber(x)).unwrap();
    let Pending::ChoosePlayer { options, .. } = engine.pending() else {
        panic!("Mind Twist must ask for its player target")
    };
    assert_eq!(options, &[me, PlayerId::new(1), PlayerId::new(2)]);
    engine
        .apply(me, PlayerAction::ChoosePlayer(victim))
        .unwrap();
    engine
}

#[test]
fn mind_twist_discards_x_random_cards_only_from_the_target_in_multiplayer() {
    let me = PlayerId::new(0);
    let victim = PlayerId::new(1);
    for x in [0, 1, 5, 10] {
        let mut engine = cast(928, x, victim);
        let before: Vec<_> = (0..3)
            .map(|seat| {
                engine
                    .state()
                    .zones
                    .list(ZoneLocation::Hand(PlayerId::new(seat)))
                    .clone()
            })
            .collect();
        let rng_before = engine.state().rng.calls();
        pass_until(&mut engine, stack_is_empty);
        let after = engine.state().zones.list(ZoneLocation::Hand(victim));
        assert_eq!(after.len(), before[1].len().saturating_sub(x as usize));
        for seat in [0, 2] {
            assert_eq!(
                engine
                    .state()
                    .zones
                    .list(ZoneLocation::Hand(PlayerId::new(seat))),
                &before[usize::from(seat)]
            );
        }
        assert!(in_graveyard(&engine, me, twist()).is_some());
        if x == 0 {
            assert_eq!(engine.state().rng.calls(), rng_before);
        }
    }
}

#[test]
fn mind_twist_random_discard_replays_deterministically_without_a_discard_choice() {
    let victim = PlayerId::new(1);
    let mut first = cast(929, 3, victim);
    let mut replay = cast(929, 3, victim);
    for engine in [&mut first, &mut replay] {
        for _ in 0..12 {
            if stack_is_empty(engine) {
                break;
            }
            let Pending::Priority { player, .. } = *engine.pending() else {
                panic!("neither player chooses random discards")
            };
            engine.apply(player, PlayerAction::PassPriority).unwrap();
        }
        assert!(stack_is_empty(engine));
    }
    assert_eq!(first.snapshot_hash(), replay.snapshot_hash());
    assert_eq!(
        first.state().zones.list(ZoneLocation::Graveyard(victim)),
        replay.state().zones.list(ZoneLocation::Graveyard(victim))
    );
}

#[test]
fn mind_twist_can_target_its_own_caster() {
    let me = PlayerId::new(0);
    let mut engine = cast(930, 1, me);
    let before = engine.state().zones.list(ZoneLocation::Hand(me)).len();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(me)).len(),
        before.saturating_sub(1)
    );
}
