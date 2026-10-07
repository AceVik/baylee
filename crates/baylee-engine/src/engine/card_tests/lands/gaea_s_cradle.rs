//! `cards/lands/gaea_s_cradle.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Gaea's Cradle: "{T}: Add {G} for each creature you control."
///
/// The pool's first counted mana ability, and the reason it is worth playing
/// rather than reading: `Effect::mana_dynamic` is `AddMana` with an `Amount`
/// instead of a number, so the amount is evaluated at resolution against the
/// board — and `Filter::YOUR_CREATURE` decides whose board. No other card in
/// the pool spells `mana_dynamic`, so every one of those two joints is
/// driven here for the first time.
///
/// Both halves are struck. Two creatures of mine and one of the opponent's
/// make two green, not three and not one; a Cradle alone makes none at all,
/// which is what says the count is a count rather than a constant.
///
/// It is activated with `ActivateAbility` and not `ActivateManaAbility`:
/// `LegalActions::mana_abilities` is the CR 305.6 shortcut for a land's
/// intrinsic mana, and a *printed* mana ability is an ordinary entry in
/// `legal.abilities` however plainly it makes mana. `choice.rs` is normative
/// on that distinction.
#[test]
fn a_counted_mana_ability_counts_your_creatures_and_nobody_elses() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(61, forest())
        .battlefield(0, &[gaea_s_cradle(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let cradle = on_battlefield(&engine, p0, gaea_s_cradle()).expect("the Cradle is out");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the pool starts empty, so what is counted below is this ability's"
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: cradle,
                ability_index: 0,
            },
        )
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        2,
        "two creatures of mine and one of theirs: \"each creature you \
         control\" is two"
    );
    assert_eq!(pool.total(), 2, "and it made nothing else");

    // The counterpart: the same land with nothing to count.
    let mut barren = Duel::new(61, forest())
        .battlefield(0, &[gaea_s_cradle()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut barren);
    reach_main_phase(&mut barren, p0);
    let cradle = on_battlefield(&barren, p0, gaea_s_cradle()).expect("the Cradle is out");
    barren
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: cradle,
                ability_index: 0,
            },
        )
        .unwrap();
    assert_eq!(
        barren.state().players[0].mana_pool.total(),
        0,
        "an empty board makes no mana at all, so the amount is read and not \
         assumed"
    );
}
