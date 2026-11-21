//! `cards/creatures/mv_4/cao_ren_wei_commander.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn cao_ren_wei_commander() -> CardIndex {
    card_index("99cbc054-2757-4c07-ad04-9ac67dad4abc")
}

fn wu_elite_cavalry() -> CardIndex {
    card_index("7dc5d729-0553-46fb-aff7-2f0d9573837b")
}

fn air_elemental() -> CardIndex {
    card_index("7744bae4-a8b7-44a5-9b4c-0048ad4cc448")
}

/// Cao Ren, Wei Commander (99cbc054-2757-4c07-ad04-9ac67dad4abc) — `{2}{B}{B}` 3/3
/// Legendary Creature — Human Soldier Warrior:
/// - Horsemanship
/// - When this creature enters, you lose 3 life.
#[test]
fn cao_ren_wei_commander_causes_you_to_lose_three_life_on_etb_and_has_horsemanship() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .battlefield(1, &[air_elemental()])
        .hand(0, &[cao_ren_wei_commander()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, cao_ren_wei_commander());
    pass_until(&mut engine, stack_is_empty);

    let cao_ren = on_battlefield(&engine, P0, cao_ren_wei_commander()).expect("Cao Ren entered");
    assert_eq!(pt(&engine, cao_ren), (3, 3));
    assert!(keywords(&engine, cao_ren).contains(KeywordSet::HORSEMANSHIP));
    assert_eq!(
        engine.state().players[0].life,
        17,
        "controller loses 3 life when Cao Ren enters"
    );
    assert_eq!(engine.state().players[1].life, 20);

    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    let blocks = attack_and_collect_blocks(&mut engine, cao_ren, P1);
    assert!(
        blocks.is_empty(),
        "horsemanship prevents creatures without horsemanship (even flyers) from blocking"
    );
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[1].life, 17);
}

#[test]
fn cao_ren_wei_commander_can_be_blocked_by_a_creature_with_horsemanship() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[cao_ren_wei_commander()])
        .battlefield(1, &[wu_elite_cavalry()])
        .start();
    keep_mulligans(&mut engine);

    let cao_ren = on_battlefield(&engine, P0, cao_ren_wei_commander()).unwrap();
    let cavalry = on_battlefield(&engine, P1, wu_elite_cavalry()).unwrap();
    let blocks = attack_and_collect_blocks(&mut engine, cao_ren, P1);
    assert!(
        blocks
            .iter()
            .any(|b| b.blocker == cavalry && b.attackers.contains(&cao_ren))
    );
}
