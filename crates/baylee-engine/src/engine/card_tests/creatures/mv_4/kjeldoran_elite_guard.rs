//! `cards/creatures/mv_4/kjeldoran_elite_guard.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::Step;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn kjeldoran_elite_guard() -> CardIndex {
    card_index("a1f6df2a-fc97-44d4-a0d1-902d8df94002")
}

/// Kjeldoran Elite Guard (a1f6df2a-fc97-44d4-a0d1-902d8df94002) — {3}{W} 2/2 Creature — Human Soldier:
/// - "{T}: Target creature gets +2/+2 until end of turn. When that creature leaves the battlefield this turn, sacrifice this creature. Activate only during combat."
#[test]
fn kjeldoran_elite_guard_activates_only_during_combat_pumps_target_and_sacrifices_when_target_leaves_this_turn()
 {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[kjeldoran_elite_guard(), index::GRIZZLY_BEARS])
        .battlefield(1, &[plains()])
        .hand(1, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let guard = on_battlefield(&engine, P0, kjeldoran_elite_guard()).expect("Guard seated");
    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).expect("Bears seated");

    assert!(
        !ability_offered(&engine, guard, 0),
        "not offered during a main phase"
    );

    pass_until(&mut engine, |e| e.state().turn.step == Step::CombatBegin);
    assert!(ability_offered(&engine, guard, 0), "offered during combat");

    activate(&mut engine, P0, kjeldoran_elite_guard(), 0);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, guard));
    assert_eq!(
        pt(&engine, bears),
        (4, 4),
        "Grizzly Bears gets +2/+2 until end of turn"
    );

    // P1 exiles Grizzly Bears this turn with Swords to Plowshares -> Kjeldoran Elite Guard is sacrificed.
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    tap_all_mana_but(&mut engine, P1, None);
    cast_from_hand(&mut engine, P1, swords_to_plowshares());
    aim_at(&mut engine, P1, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, P0, kjeldoran_elite_guard()).is_some(),
        "when the targeted creature leaves the battlefield this turn, Kjeldoran Elite Guard is sacrificed"
    );
}

#[test]
fn kjeldoran_elite_guard_stays_on_battlefield_when_target_survives_the_turn() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[kjeldoran_elite_guard(), index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).expect("Bears seated");

    pass_until(&mut engine, |e| e.state().turn.step == Step::CombatBegin);
    activate(&mut engine, P0, kjeldoran_elite_guard(), 0);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bears), (4, 4));

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, bears), (2, 2), "+2/+2 wears off at end of turn");
    assert!(
        on_battlefield(&engine, P0, kjeldoran_elite_guard()).is_some(),
        "Kjeldoran Elite Guard is not sacrificed when the target stayed on the battlefield"
    );
}
