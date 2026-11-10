//! `cards/creatures/mv_7/hornet_queen.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn hornet_queen() -> CardIndex {
    card_index("3b1f8108-6911-49e9-8f78-f950bb58cb6c")
}

/// Hornet Queen — a 2/2 flying deathtouch for {4}{G}{G}{G} that makes four 1/1
/// flying deathtouch Insects when it enters.
#[test]
fn hornet_queen_makes_four_insects() {
    let p0 = PlayerId::new(0);
    let mut engine = table7(&lots(forest(), 7), &[hornet_queen()], &[]);
    let queen = cast_and_resolve(&mut engine, hornet_queen());
    assert_body(
        &engine,
        queen,
        (2, 2),
        KeywordSet::FLYING.union(KeywordSet::DEATHTOUCH),
    );
    let insects = tokens_of(&engine, p0);
    assert_eq!(insects.len(), 4);
    for insect in insects {
        assert_body(
            &engine,
            insect,
            (1, 1),
            KeywordSet::FLYING.union(KeywordSet::DEATHTOUCH),
        );
    }
}

/// Deathtouch: the Queen blocks a 4/5 Earth Elemental, deals 2 and kills it.
#[test]
fn hornet_queen_kills_a_big_attacker_it_blocks() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[hornet_queen()])
        .battlefield(1, &[earth_elemental()])
        .start();
    keep_mulligans(&mut engine);
    let queen = on_battlefield(&engine, p0, hornet_queen()).expect("out");
    let elemental = on_battlefield(&engine, p1, earth_elemental()).expect("out");
    let _ = their_attack_blocks(&mut engine, elemental);
    block_and_finish(&mut engine, &[(queen, elemental)]);
    assert!(
        in_graveyard(&engine, p1, earth_elemental()).is_some(),
        "2 deathtouch damage is lethal to a toughness-5 creature"
    );
}
