//! `cards/instants/mv_1/erase.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Erase costs {W} and prints a line: "Exile target enchantment."
/// Exactly one enchantment is on the table — and exactly it is on the
/// target list, while the Sol Ring next to it and the Elf across as
/// permanents are still not targets, which separates
/// `Filter::ENCHANTMENT` from `Filter::Any`. The second word that matters
/// is "exile": the card must end up in its owner's exile and must *not*
/// be in the graveyard, where a destruction effect would have left it.
/// Payment is made from mana that was in the pool beforehand — the engine
/// reads payability there and not at untapped lands.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn erase_exiles_the_enchantment_it_names_and_leaves_the_rest_of_the_board() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), quiet_artifact()])
        .battlefield(1, &[their_enchantment(), llanowar_elves()])
        .hand(0, &[erase()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let victim = on_battlefield(&engine, p1, their_enchantment()).expect("the enchantment is out");
    let rock = on_battlefield(&engine, p0, quiet_artifact()).expect("my Sol Ring is out");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    let exiled_before = engine.state().zones.list(ZoneLocation::Exile(p1)).len();

    // First mana into the pool: whether a spell is castable, the engine reads
    // from the pool and not from what could still be tapped (#159).
    tap_all_mana(&mut engine, p0);
    let spell = in_hand(&engine, p0, erase()).expect("Erase is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&spell),
        "{{W}} is in the pool and an enchantment is on the battlefield, so \
         the spell is playable: {:?}",
        legal.castable
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card: spell })
        .expect("the {{W}} already in the pool pays for it");

    let Pending::ChooseTargets {
        player,
        options,
        player_options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target enchantment\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the applying seat chooses the target");
    assert_eq!((min, max), (1, 1), "exactly one enchantment");
    assert_eq!(
        options,
        vec![victim],
        "\"target enchantment\" offers the enchantment and nothing else: the \
         Sol Ring next to it and the Elf opposite are permanents and yet not \
         targets"
    );
    assert!(
        !options.contains(&rock) && !options.contains(&elf),
        "the two Bystanders are not enchantments: {options:?}"
    );
    assert!(
        player_options.is_empty(),
        "an enchantment is not a player: {player_options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![victim],
            },
        )
        .expect("die angebotene Verzauberung ist die Antwort");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, their_enchantment()).is_none(),
        "the named enchantment has left the battlefield"
    );
    assert!(
        in_graveyard(&engine, p1, their_enchantment()).is_none(),
        "\"Exile\" and not \"destroy\": a destruction effect would have put \
         it into its owner's graveyard"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p1))
            .contains(&victim),
        "and it is in exile — under its owner, not under the caster"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Exile(p1)).len(),
        exiled_before + 1,
        "one target, one exiled card"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some()
            && on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "die Beisitzer stehen unberührt"
    );
    assert!(
        in_graveyard(&engine, p0, erase()).is_some(),
        "the spell itself is in its caster's graveyard after resolution"
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        0,
        "the {{W}} from the pool is paid"
    );
}
