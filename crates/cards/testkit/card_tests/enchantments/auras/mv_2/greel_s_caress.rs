//! `cards/enchantments/auras/mv_2/greel_s_caress.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Greel's Caress is `{1}{B}` Aura with flash: "Enchant creature. Enchanted
/// creature gets -3/-0." The cast therefore happens in the *opponent's* main
/// phase, which is where flash is the difference between a spell and a
/// refusal — a sorcery could not be cast there at all. Mana is tapped before
/// the offer is read, because `castable` is filtered against the pool and not
/// against the board. The host is a 7/5 and the Elf on the caster's own side
/// is the control: `(4, 5)` on the host can only be -3/-0 landing on the
/// creature the Aura is attached to, and `(1, 1)` on the Elf says the static
/// is not "creatures".
#[test]
fn greels_caress_casts_at_flash_speed_to_weaken_only_the_creature_it_enchants() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(19, swamp())
        .battlefield(0, &[swamp(), swamp(), llanowar_elves()])
        .hand(0, &[greels_caress()])
        .battlefield(1, &[a_body_to_shrink()])
        .start();
    keep_mulligans(&mut engine);

    // Through the whole of p0's own turn, where the Aura stays in hand, and
    // into the one phase flash was printed for.
    reach_their_main_phase(&mut engine, p1);
    if matches!(engine.pending(), Pending::Priority { player, .. } if *player == p1) {
        engine.apply(p1, PlayerAction::PassPriority).unwrap();
    }
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the nonactive seat is the one being asked in the opponent's main \
         phase: {:?}",
        engine.pending()
    );

    // Mana first: an offer is computed against the pool, so a `castable`
    // claim made before the Swamps are tapped says nothing about flash.
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let card = in_hand(&engine, p0, greels_caress()).expect("the Aura is in hand");
    assert!(
        legal.castable.contains(&card),
        "flash: the Aura is castable in the opponent's main phase, where a \
         sorcery-speed card could not be: {:?}",
        legal.castable
    );

    let host = on_battlefield(&engine, p1, a_body_to_shrink()).expect("a 7/5 across the table");
    let bystander = on_battlefield(&engine, p0, llanowar_elves()).expect("a 1/1 on this side");
    assert_eq!(pt(&engine, host), (7, 5), "the printed body, untouched");

    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "an Aura asks for the creature it enchants, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the caster chooses");
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "\"enchant creature\" is every creature, on either side of the \
         table: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the creature the question offered");
    let aura = on_stack(&engine, greels_caress()).expect("the Aura spell is on the stack");

    pay_life_ward(&mut engine, p0, 7);
    pass_until(&mut engine, |e| {
        e.state()
            .object(aura)
            .is_some_and(|o| o.attached_to == Some(host))
    });

    assert!(
        on_battlefield(&engine, p0, greels_caress()).is_some(),
        "the Aura resolved onto the battlefield and stayed attached instead \
         of falling off its host"
    );
    assert_eq!(
        pt(&engine, host),
        (4, 5),
        "{{-3/-0}} on the creature it is attached to: three off the power, \
         and the toughness exactly where it was"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "and nothing for the 1/1 beside the caster: the static names the \
         attached creature and not every creature"
    );
}
