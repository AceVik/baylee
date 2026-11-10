//! `cards/creatures/mv_2/skittering_skirge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Skittering Skirge is a `{B}{B}` 3/2 Phyrexian Imp with flying and exactly
/// one additional line: "When you cast a creature spell, sacrifice this
/// creature." Three spells read the sentence from both sides. A Dark Ritual
/// is a spell and not a creature, and the Skirge stays — so it is about
/// creature spells and not about spells. An opponent's Festering Goblin is
/// a creature spell and still not one that **you** cast, and the Skirge
/// stays again. Only the creature spell of its own controller puts the
/// ability on the stack above this spell, and the Skirge is then in the
/// graveyard, while the spell underneath is still being resolved.
#[test]
fn skittering_skirge_sacrifices_itself_only_for_its_own_controllers_creature_spell() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[skittering_skirge(), dark_ritual(), festering_goblin()])
        .battlefield(1, &[swamp(), swamp()])
        .hand(1, &[festering_goblin()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Five Swamps into the pool: {B}{B} for the Skirge and one {B} each for
    // Ritual and the Goblin all come from this one main phase
    // (CR 500.5). No creature on the board produces mana, so it's five.
    assert_eq!(
        tap_all_mana(&mut engine, p0),
        5,
        "five Swamps, and nothing else on the board makes mana"
    );

    cast_with_floating(&mut engine, p0, skittering_skirge());
    pass_until(&mut engine, stack_is_empty);
    let skirge = on_battlefield(&engine, p0, skittering_skirge()).expect("der Skirge ist gelandet");
    assert_eq!(pt(&engine, skirge), (3, 2), "the body the card prints");
    assert!(
        keywords(&engine, skirge).contains(KeywordSet::FLYING),
        "and the flying that it prints"
    );

    // A spell and not a creature: the printed line says nothing about it,
    // and otherwise the Skirge has no reason to leave.
    cast_with_floating(&mut engine, p0, dark_ritual());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, skittering_skirge()).is_some(),
        "\"When you cast a creature spell\" — ein Dark Ritual ist keiner"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "das Ritual hat das eine {{B}} ausgegeben und drei zurückgelegt"
    );

    // The other half of "you": a creature spell on the table is a
    // creature spell and yet not one that this controller casts.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(tap_all_mana(&mut engine, p1), 2, "zwei Sümpfe des Gegners");
    cast_with_floating(&mut engine, p1, festering_goblin());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p1, festering_goblin()).is_some(),
        "the opponent's Goblin landed"
    );
    assert!(
        on_battlefield(&engine, p0, skittering_skirge()).is_some(),
        "and the Skirge is untouched: the ability reads *your* creature spells"
    );

    // Back to your own turn. The Swamps have untapped again in the untap
    // step, which the return value records here — without it, an offer that
    // costs nothing would not be distinguishable from an empty pool.
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        tap_all_mana(&mut engine, p0),
        5,
        "fünf Sümpfe, wieder alle untapped"
    );

    cast_with_floating(&mut engine, p0, festering_goblin());
    assert!(
        on_stack(&engine, festering_goblin()).is_some(),
        "the spell is on the stack"
    );
    assert!(
        on_battlefield(&engine, p0, skittering_skirge()).is_some(),
        "and the ability it triggered has not yet resolved — \
         the Skirge still stands, because only its trigger will fetch it"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, skittering_skirge()).is_none(),
        "\"sacrifice this creature\" — die Auslösung hat den Skirgen geholt"
    );
    assert!(
        in_graveyard(&engine, p0, skittering_skirge()).is_some(),
        "and a sacrificed permanent is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, festering_goblin()).is_some(),
        "the spell it triggered on is nevertheless resolved \
         beneath it"
    );
}
