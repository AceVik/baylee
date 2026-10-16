//! `cards/artifacts/mv_4/jade_idol.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::subtypes;

fn jade_idol() -> CardIndex {
    card_index("36b4daf5-6568-402b-a4f9-fe3ffddc847c")
}

/// Reach Through Mists: {U} Instant — Arcane.
fn reach_through_mists() -> CardIndex {
    card_index("c81ca8ff-92f3-481e-82f7-0673b6c74ea0")
}

/// Hana Kami: {G} Creature — Spirit.
fn hana_kami() -> CardIndex {
    card_index("7ac433a7-c107-46de-b36b-d7419c9ec565")
}

fn idol_after_casting(card: CardIndex, lands: &[CardIndex]) -> (Engine<RegistryLookup>, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut board = vec![jade_idol()];
    board.extend_from_slice(lands);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .hand(0, &[card])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let idol = on_battlefield(&engine, p0, jade_idol()).unwrap();
    assert!(
        !engine
            .state()
            .object(idol)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE),
        "an Idol is no creature until something makes it one"
    );
    tap_mana_except(&mut engine, p0, idol);
    cast_with_floating(&mut engine, p0, card);
    pass_until(&mut engine, stack_is_empty);
    (engine, idol)
}

/// "Whenever you cast a Spirit or Arcane spell, this artifact becomes a 4/4
/// Spirit artifact creature until end of turn." An Arcane spell does it.
#[test]
fn an_arcane_spell_animates_the_idol() {
    let (engine, idol) = idol_after_casting(reach_through_mists(), &[island()]);
    let c = engine.state().object(idol).unwrap().characteristics().clone();
    assert!(c.types.contains(TypeSet::CREATURE));
    assert!(c.types.contains(TypeSet::ARTIFACT), "still an artifact");
    assert!(c.subtypes.contains(subtypes::creature::SPIRIT));
    assert_eq!(pt(&engine, idol), (4, 4));
}

/// A Spirit spell does it too, and it wears off at end of turn.
#[test]
fn a_spirit_spell_animates_the_idol_until_end_of_turn() {
    let (mut engine, idol) = idol_after_casting(hana_kami(), &[forest()]);
    assert_eq!(pt(&engine, idol), (4, 4));
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    assert!(
        !engine
            .state()
            .object(idol)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE),
        "until end of turn"
    );
}

/// Neither Spirit nor Arcane: an Elf Druid leaves the Idol alone.
#[test]
fn any_other_spell_leaves_the_idol_alone() {
    let (engine, idol) = idol_after_casting(llanowar_elves(), &[forest()]);
    assert!(
        !engine
            .state()
            .object(idol)
            .unwrap()
            .characteristics()
            .types
            .contains(TypeSet::CREATURE)
    );
}
