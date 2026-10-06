//! `cards/enchantments/auras/mv_1/frog_tongue.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Frog Tongue — {G} Aura: "Enchant creature", "When this Aura enters, draw
/// a card", "Enchanted creature has reach."
///
/// All three printed sentences come off one cast, and each needs a bystander
/// to mean anything. `enchant creature` names no controller, so the Elf
/// across the table is on the target menu — and is precisely the creature
/// that must *not* grow the keyword afterwards. A second, bare Elf under the
/// same seat is the other control: the static reads `AttachedToBySource`, so
/// "creatures you control" would have been just as easy to write and would
/// have passed a board with only one creature on it.
#[test]
fn frog_tongue_enchants_one_creature_grants_reach_and_draws_a_card() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[frog_tongue()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert!(
        !keywords(&engine, host).contains(KeywordSet::REACH),
        "nothing enchants it yet"
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_from_hand(&mut engine, p0, frog_tongue());
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"enchant creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster chooses what it enchants");
    assert_eq!((min, max), (1, 1), "exactly one creature, asked once");
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "both creatures this seat controls may be enchanted: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "\"enchant creature\" names no controller, so the Elf across the \
         table is a legal host too: {options:?}"
    );
    assert!(
        !options.contains(&host) || options.len() == 3,
        "and those three are the whole menu"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();

    // The Aura resolving puts its own enters-trigger on the stack behind it,
    // so the board is not finished until that has resolved too.
    pass_until(&mut engine, |e| {
        stack_is_empty(e) && on_battlefield(e, p0, frog_tongue()).is_some()
    });

    let aura = on_battlefield(&engine, p0, frog_tongue()).expect("the Aura resolved");

    assert!(
        keywords(&engine, host).contains(KeywordSet::REACH),
        "\"enchanted creature has reach\""
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::REACH),
        "the Elf nobody enchanted gains nothing: the static is attached-to \
         and not \"creatures you control\""
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::REACH),
        "and the static never reaches across the table, whatever the Aura \
         could have targeted"
    );
    assert!(
        keywords(&engine, aura).is_empty(),
        "the Aura grants the keyword, it does not keep it"
    );

    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"when this Aura enters, draw a card\" — one off the top"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "the Aura left the hand and the draw put exactly one card back: a \
         missing trigger would leave one fewer"
    );
}
