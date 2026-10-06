//! `cards/instants/mv_1/quiet_purity.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Quiet Purity — {W} Instant (Arcane): "Destroy target enchantment."
///
/// One enchantment and one creature stand on each side of the table, so the
/// target offer is the whole card in a single question: it holds the
/// enchantment across the table and neither creature, which tells "target
/// enchantment" from "target permanent" and from "target creature". The {W}
/// is read off the pool rather than off the untapped Plains, and the
/// enchantment is followed into its *owner's* graveyard rather than merely
/// off the battlefield — an exile or a bounce would satisfy the first
/// reading on its own.
#[test]
fn quiet_purity_destroys_the_enchantment_across_the_table_and_no_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), llanowar_elves()])
        .battlefield(1, &[their_enchantment(), llanowar_elves()])
        .hand(0, &[quiet_purity()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let doom = on_battlefield(&engine, p1, their_enchantment()).expect("their enchantment stands");
    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves stand");
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves stand");

    // The offer is read off the pool and not off the untapped lands, so the
    // mana goes in before anything is claimed about it.
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let spell = in_hand(&engine, p0, quiet_purity()).expect("the spell is in hand");
    assert!(
        legal.castable.contains(&spell),
        "{{W}} floating pays for it: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p0, quiet_purity());
    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target enchantment\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the casting seat is the one that aims it");
    assert_eq!(
        options,
        vec![doom],
        "the one enchantment on the table: a creature is no enchantment and \
         a land is none either"
    );
    assert!(
        !options.contains(&my_elf) && !options.contains(&their_elf),
        "the two creatures are the control: {options:?}"
    );
    assert!(
        player_options.is_empty(),
        "an enchantment is an object target, so no seat is on the menu: {player_options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![doom],
            },
        )
        .expect("the enchantment the question offered is a legal answer");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, their_enchantment()).is_none(),
        "the targeted enchantment left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, their_enchantment()).is_some(),
        "and a destroyed permanent goes to its owner's graveyard — an exile \
         or a bounce would leave this empty"
    );
    assert!(
        in_graveyard(&engine, p0, their_enchantment()).is_none(),
        "under the seat that owns it and not under the one that cast the spell"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the creature beside it never moved"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "nor the one on this side, which \"destroy target enchantment\" could \
         not have named at all"
    );
}
