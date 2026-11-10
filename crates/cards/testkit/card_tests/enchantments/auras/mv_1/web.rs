//! `cards/enchantments/auras/mv_1/web.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Web — `{G}` Aura: "Enchant creature. Enchanted creature gets +0/+2 and has
/// reach."
///
/// Both printed statics are `Filter::And(&[CREATURE, AttachedToBySource])`,
/// and *that* word is the one an Aura test has to play: "attached to this" is
/// neither "creatures you control" nor "the whole table". So two Elves stand
/// under the caster and a third across it, and the single enchanted one
/// reading `(1, 3)` with reach while the other two stay `(1, 1)` and
/// keywordless is the whole of what the card says. The Aura also has to
/// arrive *attached* — an Aura that resolved and then sat loose would give
/// the numbers to nobody, which is why the attachment is read as well.
#[test]
fn web_holds_one_creature_at_one_three_with_reach_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[web()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the Web");
    assert!(
        !keywords(&engine, host).contains(KeywordSet::REACH),
        "and nothing has granted it reach yet"
    );

    cast_from_hand(&mut engine, p0, web());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"enchant creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster picks what it enchants");
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "both creatures under your own control may be enchanted: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "\"enchant creature\" is any creature, on either side of the table, \
         and never \"you control\": {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the creature the offer named");
    pass_until(&mut engine, |e| at_rest(e, p0));

    let aura = on_battlefield(&engine, p0, web()).expect("the Web resolved onto the table");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(host),
        "an Aura enters attached to the creature it was cast at"
    );

    assert_eq!(
        pt(&engine, host),
        (1, 3),
        "the enchanted creature gets +0/+2: the power is untouched and the \
         toughness is the printed 1 plus two"
    );
    assert!(
        keywords(&engine, host).contains(KeywordSet::REACH),
        "and has reach"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody enchanted is still the 1/1 it was printed as"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::REACH),
        "\"creatures you control\" would have granted this one reach too"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and the static never reaches across the table"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::REACH),
        "so the Elf across it has neither the toughness nor the keyword"
    );
    assert!(
        !keywords(&engine, aura).contains(KeywordSet::REACH),
        "the Aura grants the keyword to its host, it does not keep it"
    );
}
