#[allow(clippy::wildcard_imports)]
use super::*;

fn festering_wound() -> CardIndex {
    card_index("32638d03-c101-42f0-8a4b-8c6290257a28")
}

#[test]
fn festering_wound_accumulates_infection_counters_and_causes_life_loss() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[festering_wound()])
        .battlefield(0, &[swamp(), swamp()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();
    cast_from_hand(&mut engine, p0, festering_wound());
    aim_at(&mut engine, p0, ogre);
    pass_until(&mut engine, stack_is_empty);

    // Advance to p1's main phase, then p0's main phase (puts 1 infection counter on Festering Wound), then p1's main phase (p1 loses 1 life).
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    reach_their_main_phase(&mut engine, p1);

    assert_eq!(engine.state().players[1].life, 19);
}
