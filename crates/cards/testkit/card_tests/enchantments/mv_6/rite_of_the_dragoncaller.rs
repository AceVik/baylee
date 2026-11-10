//! `cards/enchantments/mv_6/rite_of_the_dragoncaller.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn rite_of_the_dragoncaller() -> CardIndex {
    card_index("5097f4e6-50af-4641-909f-db44abf0ce32")
}

/// Casts `spell` with `lands` under it and the Rite on the table, and lets
/// everything resolve; returns the tokens I control afterwards.
fn after_casting(spell: CardIndex, land: CardIndex) -> Vec<(i16, i16, bool)> {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[rite_of_the_dragoncaller(), land, land, land])
        .hand(0, &[spell])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, spell);
    pass_until(&mut engine, stack_is_empty);
    tokens_of(&engine, p0)
        .into_iter()
        .map(|t| {
            let (p, toughness) = pt(&engine, t);
            (
                p,
                toughness,
                keywords(&engine, t).contains(KeywordSet::FLYING),
            )
        })
        .collect()
}

/// "Whenever you cast an instant or sorcery spell, create a 5/5 red Dragon
/// creature token with flying." An instant makes one.
#[test]
fn rite_of_the_dragoncaller_makes_a_dragon_for_an_instant() {
    assert_eq!(after_casting(dark_ritual(), swamp()), [(5, 5, true)]);
}

/// A sorcery makes one too.
#[test]
fn rite_of_the_dragoncaller_makes_a_dragon_for_a_sorcery() {
    assert_eq!(
        after_casting(wheel_of_fortune(), mountain()),
        [(5, 5, true)]
    );
}

/// A creature spell makes none.
#[test]
fn rite_of_the_dragoncaller_ignores_a_creature_spell() {
    assert_eq!(after_casting(gray_ogre(), mountain()), []);
}
