//! `cards/sorceries/mv_5/lava_axe.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn lava_axe() -> CardIndex {
    card_index("387b6b07-a283-412d-94c3-f7f1dc76e858")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Lava Axe (387b6b07-a283-412d-94c3-f7f1dc76e858) — {4}{R} Sorcery:
/// "Lava Axe deals 5 damage to target player or planeswalker."
#[test]
fn lava_axe_deals_five_damage_to_target_player_or_planeswalker_not_creatures() {
    let mut engine = lk::game(
        SEED,
        &[
            mountain(),
            mountain(),
            mountain(),
            mountain(),
            mountain(),
            grizzly_bears(),
        ],
        &[grizzly_bears(), karn_the_great_creator()],
        &[lava_axe()],
        &[],
    );
    let my_bears = lk::obj(&engine, P0, grizzly_bears());
    let their_bears = lk::obj(&engine, P1, grizzly_bears());
    let karn = lk::obj(&engine, P1, karn_the_great_creator());

    cast_from_hand(&mut engine, P0, lava_axe());
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert!(player_options.contains(&P0) && player_options.contains(&P1));
    assert_eq!(options, vec![karn], "planeswalker offered, creatures not");
    assert!(!options.contains(&my_bears) && !options.contains(&their_bears));

    lk::aim(&mut engine, &[], &[P1]);
    lk::settle(&mut engine);
    assert_eq!(lk::life(&engine, P1), 15);
    assert_eq!(lk::life(&engine, P0), 20);

    let mut vs_pw = lk::game(
        SEED,
        &[mountain(), mountain(), mountain(), mountain(), mountain()],
        &[karn_the_great_creator()],
        &[lava_axe()],
        &[],
    );
    let karn_pw = lk::obj(&vs_pw, P1, karn_the_great_creator());
    tap_all_mana(&mut vs_pw, P0);
    lk::cast_at(&mut vs_pw, P0, lava_axe(), &[karn_pw], &[]);
    assert_eq!(
        lk::zone_of(&vs_pw, karn_pw),
        Zone::Graveyard,
        "5 damage removes all 5 loyalty from Karn"
    );
}
