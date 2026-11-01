//! Living Hive tramples and creates one Insect per combat damage.

use super::*;
use baylee_core::generated::subtypes;

fn living_hive() -> CardIndex {
    card_index("1aa5df06-a61d-47e8-b12d-34009854d088")
}

#[test]
fn living_hive_tramples_and_creates_tokens_for_damage_to_the_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[living_hive()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let hive = on_battlefield(&e, p0, living_hive()).unwrap();
    let ogre = on_battlefield(&e, p1, gray_ogre()).unwrap();
    assert_eq!(pt(&e, hive), (6, 6));
    assert!(keywords(&e, hive).contains(KeywordSet::TRAMPLE));
    attack_and_collect_blocks(&mut e, hive, p1);
    e.apply(
        p1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(ogre, hive)],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(e.state().players[1].life, 16, "four damage tramples over");
    let insects = e
        .state
        .battlefield_seen()
        .filter(|id| {
            *id != hive
                && e.state.object(*id).is_some_and(|object| {
                    object.controller == p0
                        && object
                            .characteristics()
                            .subtypes
                            .contains(subtypes::creature::INSECT)
                })
        })
        .count();
    assert_eq!(insects, 4);
}
