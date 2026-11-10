//! `cards/creatures/mv_5/vexing_beetle.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

/// Vexing Beetle (`30d3645f-307d-4164-a8f0-336d484b5020`) — {4}{G} Creature — Insect 3/3:
/// "This spell can't be countered.
/// This creature gets +3/+3 as long as no opponent controls a creature."
#[test]
fn vexing_beetle_cannot_be_countered_and_gets_plus_three_plus_three_when_opponent_has_no_creatures()
{
    let beetle_card = card_index("30d3645f-307d-4164-a8f0-336d484b5020");
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .battlefield(1, &[island(), island(), forest(), forest()])
        .hand(0, &[beetle_card])
        .hand(1, &[index::COUNTERSPELL, grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana_but(&mut engine, p0, None);
    cast_with_floating(&mut engine, p0, beetle_card);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();

    // Opponent casts Counterspell targeting Vexing Beetle on the stack, which does not counter it.
    let stack_spell = engine.state().zones.list(ZoneLocation::Stack)[0];
    let islands = all_on_battlefield(&engine, p1, island());
    tap_mana_where(&mut engine, p1, |id| islands.contains(&id));
    cast_with_floating(&mut engine, p1, index::COUNTERSPELL);
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![stack_spell],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let beetle = on_battlefield(&engine, p0, beetle_card).unwrap();
    assert_eq!(pt(&engine, beetle), (6, 6));

    // When opponent casts a creature on their turn, Vexing Beetle shrinks to 3/3.
    reach_their_main_phase(&mut engine, p1);
    let forests = all_on_battlefield(&engine, p1, forest());
    tap_mana_where(&mut engine, p1, |id| forests.contains(&id));
    cast_with_floating(&mut engine, p1, grizzly_bears());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, beetle), (3, 3));
}
