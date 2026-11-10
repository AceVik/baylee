//! `cards/instants/mv_1/swan_song.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Swan Song — {U} instant: "Counter target enchantment, instant, or sorcery spell.
/// Its controller creates a 2/2 blue Bird creature token with flying."
///
/// An opponent casts Dark Ritual, Swan Song counters it into the graveyard
/// and the black mana it would have made never arrives. The Bird is the
/// other half, and it is read on the **opponent**: "Its controller" is the
/// countered spell's controller, so a Bird on this side of the table would
/// be the card paying the wrong player. It is also asserted after the
/// counter has already moved the spell to a graveyard, which is where the
/// effect has to find that player.
#[test]
fn swan_song_counters_an_instant_spell_and_pays_its_caster_a_bird() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island()])
        .hand(0, &[swan_song()])
        .battlefield(1, &[swamp()])
        .hand(1, &[dark_ritual()])
        .start();
    keep_mulligans(&mut engine);

    reach_main_phase(&mut engine, p0);
    // p0 passes; p1 casts Dark Ritual during p0's main phase.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    let ritual = in_hand(&engine, p1, dark_ritual()).expect("dark ritual is in hand");
    engine
        .apply(p1, PlayerAction::CastSpell { card: ritual })
        .unwrap();

    // p1 passes priority with Dark Ritual on the stack.
    engine.apply(p1, PlayerAction::PassPriority).unwrap();

    // p0 answers: tap Island for {U} and cast Swan Song targeting Dark Ritual.
    tap_all_mana(&mut engine, p0);
    let song = in_hand(&engine, p0, swan_song()).expect("swan song is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: song })
        .unwrap();

    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected target choice for swan song, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        options,
        vec![ritual],
        "Dark Ritual is an instant spell and thus a legal target"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![ritual],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        in_graveyard(&engine, p1, dark_ritual()),
        Some(ritual),
        "Dark Ritual was countered into the graveyard"
    );
    assert_eq!(
        engine.state().players[1]
            .mana_pool
            .available(ManaColor::Black),
        0,
        "Dark Ritual never resolved, so no black mana was added"
    );
    assert_eq!(
        in_graveyard(&engine, p0, swan_song()),
        Some(song),
        "Swan Song resolved into its owner's graveyard"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "\"Its controller\" is the countered spell's controller, not the counterer"
    );
    let theirs = tokens_of(&engine, p1);
    assert_eq!(theirs.len(), 1, "one countered spell, one Bird");
    let bird = theirs[0];
    assert_eq!(pt(&engine, bird), (2, 2), "the printed 2/2");
    assert!(
        keywords(&engine, bird).contains(KeywordSet::FLYING),
        "\"with flying\""
    );
    let printed = engine
        .state()
        .object(bird)
        .expect("the Bird is on the battlefield")
        .token
        .expect("it knows which token it is");
    assert_eq!(printed.name, "Bird");
    assert!(
        printed.colors.contains(baylee_core::color::Color::Blue),
        "\"a 2/2 blue Bird\" — the pool's other Bird token is white"
    );
}
