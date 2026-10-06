//! `cards/enchantments/mv_5/nature_s_revolt.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Nature's Revolt — `{3}{G}{G}` — "All lands are 2/2 creatures that are
/// still lands."
///
/// Two statics, and each can hide the other, so every clause is read off one
/// Forest that paid for the spell: its projected type line carries both
/// `CREATURE` and `LAND`, its own basic land type is still a mana route in the
/// offer, and the combat step offers it as an attacker whose two power lands on
/// the opponent. A Forest across the table is animated by the same static
/// (`Filter::LAND` names no controller) and is the blocker that offer names,
/// while the enchantment beside them is the control that keeps "all lands" from
/// being read as "all permanents".
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn natures_revolt_animates_every_land_and_leaves_the_enchantment_beside_it_alone() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut board: Vec<CardIndex> = vec![forest(); 7];
    board.push(exploration());
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .hand(0, &[natures_revolt()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let forests = all_on_battlefield(&engine, p0, forest());
    assert_eq!(
        forests.len(),
        7,
        "six Forests for the mana and one kept back to attack with"
    );
    let kept = forests[0];
    let paid = forests[1];
    let chant = on_battlefield(&engine, p0, exploration()).expect("the Exploration is out");
    let theirs = on_battlefield(&engine, p1, forest()).expect("a Forest across the table");

    // Before anything resolves, a land is a land and nothing else: the control
    // for every reading below, since a permanent that was animated already
    // would satisfy them for a reason that has nothing to do with this card.
    assert!(
        !types(&engine, paid).contains(TypeSet::CREATURE),
        "with no Revolt on the battlefield a Forest is a land and nothing else"
    );
    assert!(
        engine
            .state()
            .object(paid)
            .expect("the Forest is an object")
            .characteristics()
            .power
            .is_none(),
        "and it carries no body for the SetPT below to be given credit for"
    );
    assert!(
        !types(&engine, theirs).contains(TypeSet::CREATURE),
        "and the Forest across the table is the same plain land"
    );

    // Six of the seven Forests pay the {3}{G}{G}; the seventh is held back
    // because it is the permanent both offers below are read off, and a Forest
    // spent for mana is tapped and could not attack.
    tap_mana_except(&mut engine, p0, kept);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six tapped Forests and the seventh kept standing: six green"
    );
    cast_with_floating(&mut engine, p0, natures_revolt());
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "{{3}}{{G}}{{G}} is five of the six, so the pool was really charged"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, natures_revolt()).is_some(),
        "the Revolt resolved onto the battlefield"
    );

    // "2/2 creatures that are still lands", read on a Forest that paid for the
    // spell — one clause each, because a permanent can be a creature with no
    // body and a body with no type.
    let animated = types(&engine, paid);
    assert!(
        animated.contains(TypeSet::CREATURE) && animated.contains(TypeSet::LAND),
        "\"creatures that are still lands\": a creature *and* still a land, and \
         not one of the two: {animated:?}"
    );
    assert_eq!(
        pt(&engine, paid),
        (2, 2),
        "the body \"are 2/2\" prints, on a permanent that had no body at all"
    );

    // `Filter::LAND` names no controller, so the Forest across the table is
    // animated by the very same static.
    let across = types(&engine, theirs);
    assert!(
        across.contains(TypeSet::CREATURE) && across.contains(TypeSet::LAND),
        "\"All lands\" is the whole table: a Forest across it is a 2/2 creature \
         that is still a land: {across:?}"
    );
    assert_eq!(
        pt(&engine, theirs),
        (2, 2),
        "and the body is the same whoever controls it"
    );

    // The negative control: an enchantment is a permanent and no land, so it is
    // neither type and has no body a leaked `Modifier::SetPT` could hide in.
    let still = types(&engine, chant);
    assert!(
        !still.contains(TypeSet::CREATURE) && still.contains(TypeSet::ENCHANTMENT),
        "\"All lands\" is not \"all permanents\": the Exploration beside them is \
         no creature: {still:?}"
    );
    assert!(
        engine
            .state()
            .object(chant)
            .expect("the Exploration is an object")
            .characteristics()
            .power
            .is_none(),
        "and it was given no body either"
    );

    // "still lands" as a behaviour rather than a type bit: a Forest that is a
    // 2/2 creature still prints its own `{G}`, so the shortcut CR 305.6 puts it
    // in carries it beside every other untapped Forest.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.mana_abilities.contains(&kept),
        "a Forest that is a creature is still a Forest: {{T}}: Add {{G}} is in \
         the offer: {:?}",
        legal.mana_abilities
    );

    // "creatures", read the same way: the permanent is an attacker the combat
    // step offers, and the animated Forest across the table is the blocker that
    // offer names — the other half of "All lands", on the other side of it.
    let blocks = attack_and_collect_blocks(&mut engine, kept, p1);
    assert!(
        blocks
            .iter()
            .any(|option| option.blocker == theirs && option.attackers.contains(&kept)),
        "the animated Forest across the table may block the animated Forest that \
         attacks, so both sides of \"All lands\" are creatures: {blocks:?}"
    );

    // Nothing blocks, so the points that land are the power the card prints —
    // and the attacker is still standing as the 2/2 creature-land it was
    // declared as, so the two came off its body and not off something the card
    // never says.
    pass_until(&mut engine, |e| e.state().players[1].life == 18);
    assert_eq!(
        engine.state().players[1].life,
        18,
        "the Forest that attacked dealt the 2 power the card prints"
    );
    assert_eq!(
        pt(&engine, kept),
        (2, 2),
        "and it is still on the battlefield as a 2/2 creature that is a land"
    );
}
