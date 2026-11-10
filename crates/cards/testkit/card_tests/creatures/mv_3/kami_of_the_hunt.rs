//! `cards/creatures/mv_3/kami_of_the_hunt.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Kami of the Hunt prints "Whenever you cast a Spirit or Arcane spell, this
/// creature gets +1/+1 until end of turn" on a 2/2, so the card is only itself
/// when the trigger reads the subtype of the spell being *cast* rather than
/// something standing on the board. A second Kami out of hand is a Spirit spell
/// and pumps the Kami that was already watching, while an Elf Druid cast by the
/// same seat in the same main phase must pump nothing — the half a trigger that
/// had lost its filter would fail. The newcomer is read beside the source
/// because `Filter::This` is the reading that matters: the +1/+1 lands on the
/// Kami that saw the spell, and the copy that has only just arrived is still
/// the printed 2/2.
#[test]
fn kami_of_the_hunt_grows_for_a_spirit_spell_and_not_for_an_elf() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(76, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                kami_of_the_hunt(),
            ],
        )
        .hand(0, &[kami_of_the_hunt(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let source = on_battlefield(&engine, p0, kami_of_the_hunt()).expect("the Kami is out");
    assert_eq!(
        pt(&engine, source),
        (2, 2),
        "the printed 2/2 before anything is cast"
    );

    // A Spirit spell: the copy out of hand is one, so the Kami already on the
    // battlefield grows. Six Forests pay {2}{G} and leave the control's {G}.
    cast_from_hand(&mut engine, p0, kami_of_the_hunt());
    pass_until(&mut engine, stack_is_empty);
    let copies = all_on_battlefield(&engine, p0, kami_of_the_hunt());
    assert_eq!(
        copies.len(),
        2,
        "the second Kami resolved onto the battlefield"
    );
    let newcomer = *copies
        .iter()
        .find(|id| **id != source)
        .expect("the copy that just arrived is a distinct object");
    assert_eq!(
        pt(&engine, source),
        (3, 3),
        "a Spirit spell was cast, so the Kami that watched it grew by one"
    );
    assert_eq!(
        pt(&engine, newcomer),
        (2, 2),
        "the +1/+1 belongs to the Kami that watched the spell, not to every \
         Spirit on the table"
    );

    // The control: an Elf Druid is a creature spell and neither a Spirit nor an
    // Arcane one, so the Kami that has just grown must not grow again.
    cast_from_hand(&mut engine, p0, llanowar_elves());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the Elf resolved, so the claim below is about a spell that was really cast"
    );
    assert_eq!(
        pt(&engine, source),
        (3, 3),
        "an Elf Druid is neither a Spirit nor an Arcane spell"
    );
}
