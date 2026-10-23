//! `cards/creatures/mv_4/mountain_titan.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::Phase;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn mountain_titan() -> CardIndex {
    card_index("160c8e94-ccf9-4058-b7fd-04529456f0fa")
}

/// Mountain Titan (160c8e94-ccf9-4058-b7fd-04529456f0fa) — {2}{B}{R} 2/2 Creature — Giant:
/// "{1}{R}{R}: Until end of turn, whenever you cast a black spell, put a +1/+1 counter on this creature."
#[test]
fn mountain_titan_gains_a_plus_one_counter_when_you_cast_a_black_spell_after_activation() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain_titan(),
                mountain(),
                mountain(),
                mountain(),
                swamp(),
                swamp(),
                forest(),
            ],
        )
        .battlefield(1, &[swamp()])
        .hand(
            0,
            &[dark_ritual(), giant_growth(), dark_ritual(), dark_ritual()],
        )
        .hand(1, &[dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let titan = on_battlefield(&engine, P0, mountain_titan()).expect("Mountain Titan seated");

    // 1. Before activating Mountain Titan's {1}{R}{R} ability, casting a black spell adds no counter.
    tap_all_mana_but(&mut engine, P0, None);
    cast_with_floating(&mut engine, P0, dark_ritual());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .object(titan)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        0,
        "casting a black spell before activating Mountain Titan puts no counter on it"
    );

    // 2. Activate Mountain Titan's {1}{R}{R} ability.
    activate(&mut engine, P0, mountain_titan(), 0);
    pass_until(&mut engine, stack_is_empty);

    // 3. Casting a green spell (Giant Growth) does not trigger Mountain Titan.
    cast_with_floating(&mut engine, P0, giant_growth());
    aim_at(&mut engine, P0, titan);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .object(titan)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        0,
        "a non-black spell you cast does not trigger Mountain Titan"
    );

    // 4. An opponent casting a black spell does not trigger Mountain Titan ("whenever you cast").
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    cast_from_hand(&mut engine, P1, dark_ritual());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .object(titan)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        0,
        "an opponent's black spell does not trigger Mountain Titan"
    );

    // 5. P0 casts a black spell (Dark Ritual): the granted trigger puts a +1/+1 counter on Mountain Titan.
    reach_main_phase(&mut engine, P0);
    cast_with_floating(&mut engine, P0, dark_ritual());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .object(titan)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1,
        "casting a black spell after activating Mountain Titan puts a +1/+1 counter on it"
    );

    // 6. "Until end of turn": on P0's next turn a black spell adds nothing.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 3 && e.state().turn.phase == Phase::FirstMain
    });
    cast_from_hand(&mut engine, P0, dark_ritual());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .object(titan)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1,
        "the granted trigger ended with the turn: a black spell on the next turn adds no counter"
    );
}
