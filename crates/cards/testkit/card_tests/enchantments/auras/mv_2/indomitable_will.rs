//! `cards/enchantments/auras/mv_2/indomitable_will.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Indomitable Will — {1}{W} Aura with flash: "Enchant creature. Enchanted
/// creature gets +1/+2."
///
/// The cast is made in the **opponent's** first main phase, which is the one
/// place the printed flash is the only thing that lets a sorcery-speed Aura
/// be played at all. The target question is then read off the whole table —
/// "enchant creature" is any creature, the Elf across it included — and the
/// pump is read off the board afterwards, where the unequipped Elf beside the
/// host and the opponent's Elf must both still be printed 1/1s, because the
/// static names the *enchanted* creature and not "creatures you control".
#[test]
fn indomitable_will_enchants_in_the_opponents_turn_and_pumps_only_its_host() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[indomitable_will()])
        .start();
    keep_mulligans(&mut engine);

    // Nothing is cast on p0's own turn: walk to the moment p0 holds priority
    // in p1's first main phase, which is the window flash opens.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert_eq!(
        engine.state().turn.active,
        p1,
        "the Aura is about to be cast on the opponent's turn"
    );

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the Aura");

    cast_from_hand(&mut engine, p0, indomitable_will());
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"enchant creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster chooses the creature it enchants");
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "both creatures under the caster are legal: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "\"enchant creature\" is any creature on the table, the opponent's \
         included: {options:?}"
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

    let aura = on_battlefield(&engine, p0, indomitable_will()).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(host),
        "the Aura lands on the creature it targeted (CR 303.4)"
    );
    assert_eq!(
        pt(&engine, host),
        (2, 3),
        "+1/+2 on the creature it enchants"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody enchanted is still the 1/1 it was printed as"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the static reaches the enchanted creature and never across the table"
    );
}
