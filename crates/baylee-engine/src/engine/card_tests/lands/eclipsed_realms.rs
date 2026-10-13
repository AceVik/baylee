//! `cards/lands/eclipsed_realms.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::subtypes::creature;

#[test]
fn eclipsed_realms_chooses_subtype_on_entry_and_produces_colored_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(308, forest())
        .hand(0, &[eclipsed_realms()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let card = in_hand(&engine, p0, eclipsed_realms()).expect("land in hand");
    engine.apply(p0, PlayerAction::PlayLand { card }).unwrap();

    let Pending::ChooseSubtype { player, options } = engine.pending().clone() else {
        panic!("expected subtype choice, got {:?}", engine.pending());
    };
    assert_eq!(player, p0);
    assert_eq!(
        options,
        vec![
            creature::ELEMENTAL,
            creature::ELF,
            creature::FAERIE,
            creature::GIANT,
            creature::GOBLIN,
            creature::KITHKIN,
            creature::MERFOLK,
            creature::TREEFOLK
        ]
    );
    let before = engine.state().snapshot_hash();
    assert!(
        engine
            .apply(p0, PlayerAction::ChooseSubtype(creature::CONSTRUCT))
            .is_err()
    );
    assert_eq!(engine.state().snapshot_hash(), before);

    engine
        .apply(p0, PlayerAction::ChooseSubtype(options[0]))
        .unwrap();

    let land = on_battlefield(&engine, p0, eclipsed_realms()).expect("land on battlefield");
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, eclipsed_realms(), 1);
    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.restricted().len(),
        1,
        "the any-colour mana is restricted to the chosen type, which is where \
         it has to land — an unrestricted blue here is the land this card was \
         until the restriction was written"
    );
    assert_eq!(pool.restricted()[0].color, ManaColor::Blue);
    assert_eq!(
        pool.available(ManaColor::Blue),
        0,
        "and not in the plain pool"
    );
    assert!(is_tapped(&engine, land));
}

/// "Spend this mana only to cast a spell of the chosen type or activate an
/// ability of a source of the chosen type." Elf chosen: an Elf creature spell
/// and an Elf permanent's ability take it; a Goblin creature spell, a Goblin
/// permanent's ability and a noncreature spell with no creature type do not.
/// A changeling noncreature spell (Crib Swap, every creature type) is "of the
/// chosen type" and does take it.
#[test]
fn eclipsed_realms_any_color_mana_pays_only_for_the_chosen_type() {
    use crate::casting::{SpendFor, pay_mana_for, spendable_units};
    use baylee_core::generated::index;
    use baylee_core::mana::ManaCost;
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                index::ECLIPSED_REALMS,
                index::LLANOWAR_ELVES,
                index::GOBLIN_SLEDDER,
            ],
        )
        .hand(
            0,
            &[
                index::LLANOWAR_ELVES,
                index::GOBLIN_SLEDDER,
                index::MIND_STONE,
                index::CRIB_SWAP,
            ],
        )
        .start();
    keep_mulligans(&mut e);
    while let Pending::ChooseSubtype { player, .. } = e.pending().clone() {
        e.apply(player, PlayerAction::ChooseSubtype(creature::ELF))
            .unwrap();
    }
    reach_main_phase(&mut e, p0);
    let realms = on_battlefield(&e, p0, index::ECLIPSED_REALMS).unwrap();
    e.dev_state_mut(p0)
        .unwrap()
        .object_mut(realms)
        .unwrap()
        .chosen_subtype = Some(creature::ELF);
    e.refresh_offer();
    activate(&mut e, p0, index::ECLIPSED_REALMS, 1);
    e.apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();
    assert_eq!(e.state().players[0].mana_pool.total(), 1);

    let elf_spell = in_hand(&e, p0, index::LLANOWAR_ELVES).unwrap();
    let goblin_spell = in_hand(&e, p0, index::GOBLIN_SLEDDER).unwrap();
    let noncreature = in_hand(&e, p0, index::MIND_STONE).unwrap();
    let changeling = in_hand(&e, p0, index::CRIB_SWAP).unwrap();
    let elf = on_battlefield(&e, p0, index::LLANOWAR_ELVES).unwrap();
    let goblin = on_battlefield(&e, p0, index::GOBLIN_SLEDDER).unwrap();

    assert_eq!(
        spendable_units(e.state(), p0, SpendFor::Spell(elf_spell)),
        1,
        "an Elf creature spell is a spell of the chosen type"
    );
    assert_eq!(
        spendable_units(e.state(), p0, SpendFor::Spell(changeling)),
        1,
        "a changeling is every creature type, so a noncreature changeling spell is of the chosen type"
    );
    assert_eq!(
        spendable_units(e.state(), p0, SpendFor::Ability(elf)),
        1,
        "an ability of an Elf permanent"
    );
    for (what, why) in [
        (SpendFor::Spell(goblin_spell), "a Goblin creature spell"),
        (
            SpendFor::Spell(noncreature),
            "a noncreature spell with no creature type",
        ),
        (
            SpendFor::Ability(goblin),
            "an ability of a Goblin permanent",
        ),
        (
            SpendFor::Other,
            "a payment that is neither a spell nor an ability",
        ),
    ] {
        assert_eq!(spendable_units(e.state(), p0, what), 0, "refused: {why}");
        let before = e.state().players[0].mana_pool.clone();
        assert!(
            pay_mana_for(
                e.dev_state_mut(p0).unwrap(),
                p0,
                what,
                &ManaCost::parse("{1}")
            )
            .is_none(),
            "no payment of {{1}}: {why}"
        );
        assert_eq!(
            e.state().players[0].mana_pool,
            before,
            "the refused payment is atomic"
        );
    }
}
