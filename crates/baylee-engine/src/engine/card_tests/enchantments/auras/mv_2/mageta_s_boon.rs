//! `cards/enchantments/auras/mv_2/mageta_s_boon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mageta's Boon is a {1}{W} Aura with flash — "Enchant creature; enchanted
/// creature gets +1/+2". It is cast here in the *opponent's* main phase,
/// which is the only place its flash is observable: a sorcery-speed Aura is
/// absent from `castable` there. It enchants one of two Elves while a third
/// stands across the table, so the +1/+2 has to land on the creature the Aura
/// is attached to rather than on "creatures you control" or on every creature
/// in the game.
#[test]
fn magetas_boon_flashes_in_and_pumps_only_the_creature_it_enchants() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[magetas_boon()])
        .start();
    keep_mulligans(&mut engine);

    let mine = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(mine.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (mine[0], mine[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the Aura");

    // Onto the opponent's main phase, and then to the priority p0 holds
    // inside it — the two Plains are untapped and stay that way until the
    // Aura is paid for.
    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });

    // Mana before the claim: `castable` is filtered against the pool, not
    // against the untapped lands sitting beside it.
    tap_all_mana(&mut engine, p0);
    let aura = in_hand(&engine, p0, magetas_boon()).expect("the Aura is in hand");
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat about to cast the instant-speed Aura");
    assert!(
        legal.castable.contains(&aura),
        "flash: a {{1}}{{W}} Aura is castable in the opponent's main phase, \
         where a sorcery-speed one could not be: {:?}",
        legal.castable
    );

    engine
        .apply(p0, PlayerAction::CastSpell { card: aura })
        .expect("flash makes the cast legal at instant speed");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "\"Enchant creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&host) && options.contains(&bystander) && options.contains(&theirs),
        "\"Enchant creature\" is any creature, on either side of the table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the Elf the question offered");
    pass_until(&mut engine, stack_is_empty);

    let enchant = on_battlefield(&engine, p0, magetas_boon()).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(enchant).and_then(|o| o.attached_to),
        Some(host),
        "an Aura enters attached to the creature it targeted"
    );
    assert_eq!(pt(&engine, host), (2, 3), "+1/+2 on the enchanted creature");
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf nobody enchanted is still the 1/1 it was printed as"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and the static reaches no creature it is not attached to"
    );
}
