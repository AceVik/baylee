//! `cards/creatures/mv_5/kjeldoran_royal_guard.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn kjeldoran_royal_guard() -> CardIndex {
    card_index("8137e765-4df9-469f-a527-dee91d58fb7e")
}

/// Kjeldoran Royal Guard (8137e765-4df9-469f-a527-dee91d58fb7e) — {3}{W}{W} 2/5 Creature — Human Soldier:
/// - "{T}: All combat damage that would be dealt to you by unblocked creatures this turn is dealt to this creature instead."
#[test]
fn kjeldoran_royal_guard_redirects_damage_from_unblocked_attackers_to_itself_and_not_from_blocked_tramplers()
 {
    let mut engine = lk::game(
        SEED,
        &[kjeldoran_royal_guard(), llanowar_elves()],
        &[index::HILL_GIANT, index::WAR_MAMMOTH],
        &[],
        &[],
    );
    let guard = lk::obj(&engine, P0, kjeldoran_royal_guard());
    let elf = lk::obj(&engine, P0, llanowar_elves());
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);
    let mammoth = lk::obj(&engine, P1, index::WAR_MAMMOTH);

    // Advance to P1's turn.
    reach_their_main_phase(&mut engine, P1);

    // P1 attacks with 3/3 Hill Giant (unblocked) and 3/3 trample War Mammoth (blocked by 1/1 Llanowar Elves).
    let _ = lk::attack(&mut engine, &[giant, mammoth]);
    engine
        .apply(
            P0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elf, mammoth)],
            },
        )
        .unwrap();

    // After blockers are declared, P0 activates Kjeldoran Royal Guard's {T} ability.
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    activate(&mut engine, P0, kjeldoran_royal_guard(), 0);
    lk::through_combat(&mut engine);

    assert!(is_tapped(&engine, guard));
    assert_eq!(
        engine.state().object(guard).unwrap().damage,
        3,
        "3 combat damage from unblocked Hill Giant was redirected to the 2/5 Kjeldoran Royal Guard"
    );
    assert_eq!(
        engine.state().players[0].life,
        18,
        "2 trample combat damage from blocked War Mammoth was dealt to P0 (not redirected), while Hill Giant's 3 was redirected"
    );
}
