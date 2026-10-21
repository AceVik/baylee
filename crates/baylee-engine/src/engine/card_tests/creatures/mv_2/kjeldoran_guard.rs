//! `cards/creatures/mv_2/kjeldoran_guard.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::Step;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn kjeldoran_guard() -> CardIndex {
    card_index("bd699cf8-56c7-4375-920e-8dd22c83a82c")
}

/// Kjeldoran Guard (bd699cf8-56c7-4375-920e-8dd22c83a82c) — {1}{W} 1/1 Creature — Human Soldier:
/// - "{T}: Target creature gets +1/+1 until end of turn. When that creature leaves the battlefield this turn, sacrifice this creature. Activate only during combat and only if defending player controls no snow lands."
#[test]
fn kjeldoran_guard_activates_during_combat_when_defender_has_no_snow_land_and_sacrifices_when_target_leaves()
 {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                kjeldoran_guard(),
                index::GRIZZLY_BEARS,
                index::SNOW_COVERED_PLAINS,
            ],
        )
        .battlefield(1, &[plains()])
        .hand(1, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let guard = on_battlefield(&engine, P0, kjeldoran_guard()).expect("Guard seated");
    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).expect("Bears seated");

    assert!(
        !ability_offered(&engine, guard, 0),
        "not offered outside combat"
    );

    // P0's own Snow-Covered Plains does not belong to the defending player (P1).
    pass_until(&mut engine, |e| e.state().turn.step == Step::CombatBegin);
    assert!(
        ability_offered(&engine, guard, 0),
        "offered during combat when defending player P1 controls no snow lands"
    );

    activate(&mut engine, P0, kjeldoran_guard(), 0);
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, guard));
    assert_eq!(pt(&engine, bears), (3, 3));

    // P1 exiles Grizzly Bears this turn -> Kjeldoran Guard is sacrificed.
    engine.apply(P0, PlayerAction::PassPriority).unwrap();
    tap_all_mana_but(&mut engine, P1, None);
    cast_from_hand(&mut engine, P1, swords_to_plowshares());
    aim_at(&mut engine, P1, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, P0, kjeldoran_guard()).is_some(),
        "Kjeldoran Guard is sacrificed when the targeted creature leaves the battlefield this turn"
    );
}

#[test]
fn kjeldoran_guard_is_not_offered_while_defending_player_controls_a_snow_land() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[kjeldoran_guard(), index::GRIZZLY_BEARS])
        .battlefield(1, &[index::SNOW_COVERED_ISLAND])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let guard = on_battlefield(&engine, P0, kjeldoran_guard()).expect("Guard seated");
    pass_until(&mut engine, |e| e.state().turn.step == Step::CombatBegin);
    assert!(
        !ability_offered(&engine, guard, 0),
        "not offered during combat when defending player controls a snow land"
    );
}
