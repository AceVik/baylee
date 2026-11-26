//! `cards/creatures/mv_3/soltari_visionary.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as kit;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn soltari_visionary() -> CardIndex {
    card_index("7971f5c2-97b7-441f-ad77-954f43ede0a1")
}

fn plains() -> CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn pacifism() -> CardIndex {
    card_index("5f5e0b10-c8cf-450c-bfd3-bcb0528ec330")
}

#[test]
fn soltari_visionary_has_shadow_and_destroys_enchantment_on_player_damage() {
    let mut e = Duel::new(SEED, plains())
        .battlefield(0, &[soltari_visionary()])
        .battlefield(1, &[grizzly_bears(), plains(), plains()])
        .hand(1, &[pacifism()])
        .start();
    keep_mulligans(&mut e);

    // P1 enchants their own Bears just so P1 controls an enchantment on the battlefield.
    reach_their_main_phase(&mut e, P1);
    let p1_bears = on_battlefield(&e, P1, grizzly_bears()).unwrap();
    kit::float(
        &mut e,
        P1,
        &[(ManaColor::White, 1), (ManaColor::Colorless, 1)],
    );
    kit::cast_at(&mut e, P1, pacifism(), &[p1_bears], &[]);
    pass_until(&mut e, stack_is_empty);

    // P0 turn: attack with Soltari Visionary; Bears cannot block (Shadow), P1 takes 2 and Pacifism is destroyed.
    reach_their_main_phase(&mut e, P0);
    let vis = on_battlefield(&e, P0, soltari_visionary()).unwrap();
    assert!(keywords_of(&e, vis).contains(KeywordSet::SHADOW));
    let block_offer = kit::attack(&mut e, &[vis]);
    assert!(!kit::may_block(&block_offer, p1_bears, vis));
    e.apply(P1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let pac_bf = on_battlefield(&e, P1, pacifism()).unwrap();
    kit::aim(&mut e, &[pac_bf], &[]);
    pass_until(&mut e, stack_is_empty);

    assert_eq!(e.state().players[1].life, 18);
    assert!(on_battlefield(&e, P1, pacifism()).is_none());
}
