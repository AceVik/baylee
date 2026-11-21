//! `cards/sorceries/mv_5/burning_fields.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn burning_fields() -> CardIndex {
    card_index("b3b58231-d532-423b-91d0-78d9c0f66f0f")
}

/// Burning Fields (b3b58231-d532-423b-91d0-78d9c0f66f0f) — {4}{R} Sorcery:
/// "Burning Fields deals 5 damage to target opponent or planeswalker."
#[test]
fn burning_fields_deals_five_damage_to_opponent_or_planeswalker_only() {
    let mut lands = vec![mountain(); 5];
    lands.push(aurochs());
    let mut engine = lk::game(
        SEED,
        &lands,
        &[aurochs(), karn_the_great_creator()],
        &[burning_fields()],
        &[],
    );
    let my_beast = lk::obj(&engine, P0, aurochs());
    let their_beast = lk::obj(&engine, P1, aurochs());
    let karn = lk::obj(&engine, P1, karn_the_great_creator());

    cast_from_hand(&mut engine, P0, burning_fields());
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert_eq!(player_options, vec![P1], "only opponents may be targeted");
    assert_eq!(options, vec![karn], "planeswalker offered, creatures not");
    assert!(!options.contains(&my_beast) && !options.contains(&their_beast));

    lk::aim(&mut engine, &[], &[P1]);
    lk::settle(&mut engine);
    assert_eq!(lk::life(&engine, P1), 15);
    assert_eq!(lk::life(&engine, P0), 20);

    let mut vs_pw = lk::game(
        SEED,
        &[mountain(); 5],
        &[karn_the_great_creator()],
        &[burning_fields()],
        &[],
    );
    let karn_pw = lk::obj(&vs_pw, P1, karn_the_great_creator());
    tap_all_mana(&mut vs_pw, P0);
    lk::cast_at(&mut vs_pw, P0, burning_fields(), &[karn_pw], &[]);
    assert_eq!(lk::zone_of(&vs_pw, karn_pw), Zone::Graveyard);
}
