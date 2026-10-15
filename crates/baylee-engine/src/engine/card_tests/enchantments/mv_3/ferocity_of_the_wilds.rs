#[allow(clippy::wildcard_imports)] // Shared card-test harness.
use super::*;
use baylee_core::generated::index::{ALERT_SHU_INFANTRY, METALLIC_SLIVER};

fn subject() -> CardIndex {
    card_index("c75bdc5f-22f2-4558-bb8f-5a53ff7ab1ac")
}

// Scenario: only our attacking non-Human gains +1 power and trample;
// own attacking Human, nonattacking non-Human, and opponent creatures do not.
#[test]
fn ferocity_arms_only_our_attacking_nonhumans() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                subject(),
                llanowar_elves(),
                ALERT_SHU_INFANTRY,
                METALLIC_SLIVER,
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut e);
    assert!(walk_to_own_main(&mut e, p0));
    let elf = on_battlefield(&e, p0, llanowar_elves()).unwrap();
    let human = on_battlefield(&e, p0, ALERT_SHU_INFANTRY).unwrap();
    let idle = on_battlefield(&e, p0, METALLIC_SLIVER).unwrap();
    let theirs = on_battlefield(&e, p1, llanowar_elves()).unwrap();
    assert_eq!(pt(&e, elf), (1, 1));
    pass_until(&mut e, |game| {
        matches!(game.pending(), Pending::ChooseAttackers { .. })
    });
    e.apply(
        p0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(elf, Defender::Player(p1)), (human, Defender::Player(p1))],
        },
    )
    .unwrap();
    assert_eq!(pt(&e, elf), (2, 1));
    assert!(keywords(&e, elf).contains(KeywordSet::TRAMPLE));
    assert_eq!(pt(&e, human), (2, 2));
    assert!(!keywords(&e, human).contains(KeywordSet::TRAMPLE));
    assert_eq!(pt(&e, idle), (1, 1));
    assert!(!keywords(&e, idle).contains(KeywordSet::TRAMPLE));
    assert_eq!(pt(&e, theirs), (1, 1));
    pass_until(&mut e, |game| {
        matches!(game.state().turn.phase, Phase::Ending)
    });
    assert_eq!(pt(&e, elf), (1, 1));
    assert!(!keywords(&e, elf).contains(KeywordSet::TRAMPLE));
    reach_their_main_phase(&mut e, p1);
    pass_until(
        &mut e,
        |game| matches!(game.pending(), Pending::ChooseAttackers { player, .. } if *player == p1),
    );
    e.apply(
        p1,
        PlayerAction::DeclareAttackers {
            attackers: vec![(theirs, Defender::Player(p0))],
        },
    )
    .unwrap();
    assert_eq!(pt(&e, theirs), (1, 1));
    assert!(!keywords(&e, theirs).contains(KeywordSet::TRAMPLE));
}
