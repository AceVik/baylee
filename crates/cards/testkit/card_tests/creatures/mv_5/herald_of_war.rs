//! `cards/creatures/mv_5/herald_of_war.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn herald_of_war() -> CardIndex {
    card_index("1228bd6f-565d-4ad5-bd24-e5aa1ead5dee")
}

fn inspiring_overseer_card() -> CardIndex {
    card_index("d646e42b-5635-4798-b633-29c093b66a55")
}

#[test]
fn herald_of_war_attack_counter_reduces_angel_and_human_spells_by_one() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let human = baylee_core::generated::index::CABAL_INQUISITOR;
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[herald_of_war(), plains(), plains(), swamp()])
        .hand(0, &[inspiring_overseer_card(), human, gray_ogre()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let herald = on_battlefield(&e, p0, herald_of_war()).expect("the Herald");
    assert!(keywords(&e, herald).contains(KeywordSet::FLYING));
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    e.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(herald, Defender::Player(p1))],
        },
    )
    .expect("the Herald attacks");
    pass_until(&mut e, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(counters_on(&e, herald, CounterKind::P1P1), 1);

    let lands = all_on_battlefield(&e, p0, plains());
    tap_mana_where(&mut e, p0, |id| lands.contains(&id));
    let legal = priority_offer(&e);
    let angel = in_hand(&e, p0, inspiring_overseer_card()).expect("Angel in hand");
    let ogre = in_hand(&e, p0, gray_ogre()).expect("non-Angel/Human control");
    assert!(
        legal.castable.contains(&angel),
        "the Angel costs one generic mana less"
    );
    assert!(!legal.castable.contains(&ogre), "an Ogre gets no reduction");
    cast_with_floating(&mut e, p0, inspiring_overseer_card());
    pass_until(&mut e, stack_is_empty);

    tap_all_mana(&mut e, p0);
    let cleric = in_hand(&e, p0, human).expect("Human in hand");
    assert!(
        priority_offer(&e).castable.contains(&cleric),
        "the Human costs only its colored mana"
    );
    assert!(!priority_offer(&e).castable.contains(&ogre));
    cast_with_floating(&mut e, p0, human);
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, p0, inspiring_overseer_card()).is_some());
    assert!(on_battlefield(&e, p0, human).is_some());
    assert!(in_hand(&e, p0, gray_ogre()).is_some());
}

/// "For each +1/+1 counter": with none, no reduction. The attack test's
/// control was an Ogre that costs red on a board with no red, so a flat
/// {1} off every Angel would have passed it.
#[test]
fn herald_of_war_without_counters_reduces_nothing() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[herald_of_war(), plains(), plains()])
        .hand(0, &[inspiring_overseer_card()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let herald = on_battlefield(&e, p0, herald_of_war()).expect("the Herald");
    assert_eq!(counters_on(&e, herald, CounterKind::P1P1), 0);
    tap_all_mana(&mut e, p0);
    let angel = in_hand(&e, p0, inspiring_overseer_card()).expect("Angel in hand");
    assert!(
        !priority_offer(&e).castable.contains(&angel),
        "{{2}}{{W}} on two white mana"
    );
}
