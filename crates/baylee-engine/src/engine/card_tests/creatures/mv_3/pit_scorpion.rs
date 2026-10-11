//! Printed behavior and adversarial resolution cases.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

#[test]
fn scorpion_poisons_any_damaged_player_once_for_combat_or_noncombat_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for (player, combat, amount) in [(p0, false, 3), (p1, false, 2), (p1, true, 1), (p1, true, 0)] {
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[index::PIT_SCORPION, index::DOUBLING_SEASON])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let scorpion = on_battlefield(&e, p0, index::PIT_SCORPION).unwrap();
        crate::engine::card_tests::history_helpers::damage_player(
            &mut e, scorpion, player, amount, combat,
        );
        e.apply(p0, PlayerAction::PassPriority).unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            e.state().players[usize::from(player.get())].poison,
            u16::from(amount > 0)
        );
        assert_eq!(
            e.state().players[usize::from(player.get())].life,
            20 - i32::try_from(amount).unwrap()
        );
    }
}
