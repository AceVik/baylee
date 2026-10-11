//! `cards/lands/legendary/minas_tirith.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Minas Tirith prints `Minas Tirith enters tapped unless you control a legendary creature`, `{T}: Add {W}`,
/// and `{1}{W}, {T}: Draw a card. Activate only if you attacked with two or more creatures this turn.`
/// With controlled legendary creature `thorin_oakenshield` on the battlefield, Minas Tirith enters untapped
/// and taps for `{W}` at ability index 0 while omitting the draw ability.
#[test]
fn minas_tirith_enters_untapped_with_legendary_creature_and_taps_for_white() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[thorin_oakenshield()])
        .hand(0, &[minas_tirith()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, minas_tirith());
    assert!(!entered_tapped(&engine, land));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority");
    };
    let offered: Vec<(ObjectId, u32)> = legal
        .abilities
        .iter()
        .copied()
        .filter(|(source, _)| *source == land)
        .collect();
    assert_eq!(offered, vec![(land, 0)]);

    activate(&mut engine, p0, minas_tirith(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::White), 1);
    assert!(is_tapped(&engine, land));
}

#[test]
fn minas_remembers_its_controllers_declared_attackers_even_after_they_leave() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for count in 1..=2 {
        let mut e = Duel::new(SEED, forest())
            .battlefield(
                0,
                &[minas_tirith(), index::GRIZZLY_BEARS, index::GRIZZLY_BEARS],
            )
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let land = on_battlefield(&e, p0, minas_tirith()).unwrap();
        let bears = all_on_battlefield(&e, p0, index::GRIZZLY_BEARS);
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseAttackers { .. })
        });
        e.apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: bears
                    .iter()
                    .take(count)
                    .map(|&id| (id, Defender::Player(p1)))
                    .collect(),
            },
        )
        .unwrap();
        pass_until(&mut e, |e| {
            e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
        });
        bury(&mut e, &[bears[0]]);
        e.dev_state_mut(p0).unwrap().players[0]
            .mana_pool
            .add(ManaColor::White, 2);
        e.dev_state_mut(p0).unwrap().set_tapped(land, false);
        e.refresh_offer();
        let Pending::Priority { legal, .. } = e.pending() else {
            panic!("priority")
        };
        assert_eq!(legal.abilities.contains(&(land, 1)), count == 2);
        if count == 2 {
            let hand = e.state().zones.list(ZoneLocation::Hand(p0)).len();
            activate(&mut e, p0, minas_tirith(), 1);
            pass_until(&mut e, stack_is_empty);
            assert_eq!(e.state().zones.list(ZoneLocation::Hand(p0)).len(), hand + 1);
        }
        reach_their_main_phase(&mut e, p1);
        e.dev_state_mut(p0).unwrap().set_tapped(land, false);
        e.dev_state_mut(p0).unwrap().players[0]
            .mana_pool
            .add(ManaColor::White, 2);
        e.apply(p1, PlayerAction::PassPriority).unwrap();
        let Pending::Priority { legal, .. } = e.pending() else {
            panic!("priority")
        };
        assert!(!legal.abilities.contains(&(land, 1)));
    }
}
