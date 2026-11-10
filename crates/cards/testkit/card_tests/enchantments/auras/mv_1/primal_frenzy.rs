//! `cards/enchantments/auras/mv_1/primal_frenzy.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Primal Frenzy — {G} — Aura: "Enchant creature. Enchanted creature has
/// trample."
///
/// The one line the card prints is a static on `Filter::AttachedToBySource`,
/// so the only reading worth playing is the one that tells the creature the
/// Aura *is on* from every other creature in the game — which is why both
/// bystanders are real creatures rather than scenery: an Elf beside the host
/// under the same seat and an Elf across the table. A reading of "creatures
/// you control" would leave two tramplers, and a reading with no filter at
/// all three; exactly one trampler is the only answer that reads both the
/// filter and the word "enchanted".
///
/// The menu read before the answer is the other half: "enchant creature" is
/// a target chosen as the Aura is cast (CR 601.2c) with no controller in it,
/// so all three creatures are on it and no Forest is.
#[test]
fn primal_frenzy_grants_trample_to_the_creature_it_enchants_and_to_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[primal_frenzy()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert!(
        !keywords(&engine, host).contains(KeywordSet::TRAMPLE),
        "nothing is enchanted yet"
    );

    cast_from_hand(&mut engine, p0, primal_frenzy());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"enchant creature\" is a target chosen as the Aura is cast, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&host) && options.contains(&bystander) && options.contains(&theirs),
        "\"creature\" is read with no controller in it: {options:?}"
    );
    assert_eq!(
        options.len(),
        3,
        "and those three are the whole menu — the Forests are no creatures: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    let frenzy = on_battlefield(&engine, p0, primal_frenzy()).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(frenzy).and_then(|o| o.attached_to),
        Some(host),
        "and it is attached to the creature it was aimed at"
    );
    assert!(
        types(&engine, frenzy).contains(TypeSet::ENCHANTMENT),
        "an Aura is an enchantment once it is on the battlefield: {:?}",
        types(&engine, frenzy)
    );

    assert!(
        keywords(&engine, host).contains(KeywordSet::TRAMPLE),
        "enchanted creature has trample"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::TRAMPLE),
        "the Elf nobody enchanted is still the 1/1 it was printed as"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::TRAMPLE),
        "the static reaches the enchanted creature and never across the table"
    );
    assert!(
        !keywords(&engine, frenzy).contains(KeywordSet::TRAMPLE),
        "the Aura grants the keyword, it does not keep it"
    );
}
