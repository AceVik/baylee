//! `cards/enchantments/mv_4/daxos_s_torment.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn daxos_s_torment() -> CardIndex {
    card_index("93ef3098-d1c0-4b05-9d28-1afa133bfc4f")
}

fn aether_sting() -> CardIndex {
    card_index("61105cb5-d7a1-4021-a006-dd1b947dfa68")
}

/// What Daxos's Torment is, projected, at this moment.
fn is_a_demon_creature(engine: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    let chars = engine.state().object(id).expect("object").characteristics();
    chars.types.contains(TypeSet::CREATURE)
        && chars.types.contains(TypeSet::ENCHANTMENT)
        && chars
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::DEMON)
        && chars.power == Some(5)
        && chars.toughness == Some(5)
        && chars
            .keywords
            .contains(baylee_cards_dsl::KeywordSet::FLYING)
        && chars.keywords.contains(baylee_cards_dsl::KeywordSet::HASTE)
}

/// "Constellation — Whenever this enchantment or another enchantment you
/// control enters, this enchantment becomes a 5/5 Demon creature with flying
/// and haste in addition to its other types until end of turn.": cast, it
/// triggers on its own entry and is a creature that turn only.
#[test]
fn daxos_s_torment_animates_itself_when_it_enters_until_end_of_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[daxos_s_torment()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    float(&mut engine, p0, ManaColor::Black, 4);
    cast_with_floating(&mut engine, p0, daxos_s_torment());
    pass_until(&mut engine, stack_is_empty);
    let daxos = on_battlefield(&engine, p0, daxos_s_torment()).expect("resolved");
    assert!(
        is_a_demon_creature(&engine, daxos),
        "a 5/5 flying haste Demon"
    );

    let turn = engine.state().turn.number;
    pass_until(&mut engine, |e| e.state().turn.number > turn);
    assert!(
        !types(&engine, daxos).contains(TypeSet::CREATURE),
        "until end of turn: an enchantment again on the next turn"
    );
}

/// Another enchantment of mine entering animates it too; an opponent's
/// enchantment entering does not.
#[test]
fn daxos_s_torment_animates_for_my_other_enchantments_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[daxos_s_torment()])
        .hand(0, &[aether_sting()])
        .hand(1, &[aether_sting()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p1);
    let daxos = on_battlefield(&engine, p0, daxos_s_torment()).expect("seated");

    float(&mut engine, p1, ManaColor::Red, 4);
    cast_with_floating(&mut engine, p1, aether_sting());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !types(&engine, daxos).contains(TypeSet::CREATURE),
        "an opponent's enchantment is not mine"
    );

    reach_their_main_phase(&mut engine, p0);
    float(&mut engine, p0, ManaColor::Red, 4);
    cast_with_floating(&mut engine, p0, aether_sting());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        is_a_demon_creature(&engine, daxos),
        "my other enchantment entered"
    );
}
