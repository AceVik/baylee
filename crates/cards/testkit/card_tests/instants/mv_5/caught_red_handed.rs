//! Caught Red-Handed, played against a counterspell.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;
use baylee_engine::event::GameEvent;

#[test]
fn caught_red_handed_changes_control_before_untapping_and_suspects_the_creature() {
    let caught = card_index("484b3586-c0ed-4793-a3c0-e99f265e092d");
    let (me, them) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[caught])
        .battlefield(0, &[mountain(); 5])
        .hand(1, &[counterspell()])
        .battlefield(1, &[index::BOGGART_BRUTE, island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, me);

    let brute = on_battlefield(&engine, them, index::BOGGART_BRUTE).unwrap();
    let printed = keywords(&engine, brute);
    assert!(printed.contains(KeywordSet::MENACE));
    assert!(!printed.contains(KeywordSet::CANT_BLOCK));
    assert!(engine.dev_state_mut(them).unwrap().set_tapped(brute, true));
    engine.refresh_offer();

    cast_from_hand(&mut engine, me, caught);
    aim_at(&mut engine, me, brute);
    let spell = top_of_stack(&engine);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == them),
    );
    cast_from_hand(&mut engine, them, counterspell());
    aim_at(&mut engine, them, spell);
    pass_until(&mut engine, stack_is_empty);

    let journal = engine.journal().entries();
    let control = journal
        .iter()
        .position(|entry| {
            matches!(entry.event, GameEvent::ControllerChanged { object, old, new }
                if object == brute && old == them && new == me)
        })
        .expect("the target changed control");
    let untap = journal
        .iter()
        .position(|entry| {
            matches!(entry.event, GameEvent::ObjectUntapped { object, .. } if object == brute)
        })
        .expect("the target untapped");
    assert!(
        control < untap,
        "the creature changes control before it untaps"
    );
    assert_eq!(
        on_battlefield(&engine, me, index::BOGGART_BRUTE),
        Some(brute)
    );
    assert!(!is_tapped(&engine, brute));
    let borrowed = keywords(&engine, brute);
    assert!(borrowed.contains(KeywordSet::HASTE));
    assert!(borrowed.contains(KeywordSet::MENACE));
    assert!(borrowed.contains(KeywordSet::CANT_BLOCK));

    reach_their_main_phase(&mut engine, them);
    assert_eq!(
        on_battlefield(&engine, them, index::BOGGART_BRUTE),
        Some(brute)
    );
    let returned = keywords(&engine, brute);
    assert!(!returned.contains(KeywordSet::HASTE));
    assert!(returned.contains(KeywordSet::MENACE));
    assert!(returned.contains(KeywordSet::CANT_BLOCK));
}
