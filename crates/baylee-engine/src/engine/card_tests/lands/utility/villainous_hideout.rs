//! `cards/lands/utility/villainous_hideout.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::generated::index;

/// Villainous Hideout: "{T}: Add {C}." / "{T}: Add one mana of any color. Spend this mana only to cast a Villain spell..." / "{3}, {T}: Target Villain you control connives."
/// Activating ability 1 adds one restricted mana of any chosen color to `pool.restricted()`.
#[test]
fn villainous_hideout_adds_restricted_villain_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(231, forest())
        .battlefield(0, &[villainous_hideout()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hideout = on_battlefield(&engine, p0, villainous_hideout()).expect("Hideout deployed");
    activate(&mut engine, p0, villainous_hideout(), 1);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert_eq!(options.len(), 5);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 0);
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::Red);
    assert!(is_tapped(&engine, hideout));
}

/// p0's main phase with the Hideout (and, when asked, Maskwood Nexus, the only
/// way this pool has to make a card a Villain) and `spell` in hand; the Hideout
/// has been tapped for one green mana, which is then the whole pool.
fn hideout_green_mana(nexus: bool, spell: CardIndex) -> (Engine<RegistryLookup>, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut board = vec![villainous_hideout()];
    if nexus {
        board.push(index::MASKWOOD_NEXUS);
    }
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .hand(0, &[spell])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    activate(&mut e, p0, villainous_hideout(), 1);
    e.apply(p0, PlayerAction::ChooseColor(ManaColor::Green))
        .unwrap();
    let pool = &e.state().players[0].mana_pool;
    assert_eq!(
        (pool.total(), pool.restricted().len()),
        (1, 1),
        "control: the whole pool is the Hideout's one restricted green"
    );
    let card = in_hand(&e, p0, spell).expect("the spell is in hand");
    (e, card)
}

/// "Spend this mana only to cast a Villain spell": with Maskwood Nexus the
/// Llanowar Elves in hand are a Villain creature spell, and the restricted
/// green pays its `{G}`.
#[test]
fn villainous_hideout_mana_casts_a_villain_spell() {
    let (e, elves) = hideout_green_mana(true, index::LLANOWAR_ELVES);
    assert_eq!(
        crate::casting::can_cast(e.state(), &RegistryLookup, PlayerId::new(0), elves),
        Ok(()),
        "a Villain spell may be cast with the Hideout's mana"
    );
}

/// The restriction's negative half: the same Elves and the same mana, but
/// without a Villain anywhere (an Elf is not a Villain), cannot be cast.
#[test]
fn villainous_hideout_mana_does_not_cast_a_spell_that_is_not_a_villain() {
    let (e, elves) = hideout_green_mana(false, index::LLANOWAR_ELVES);
    assert_eq!(
        crate::casting::can_cast(e.state(), &RegistryLookup, PlayerId::new(0), elves),
        Err(crate::casting::CastError::NotEnoughMana),
        "an Elf is no Villain: the restricted green pays for nothing"
    );
}

/// "Villain spell" is a creature spell with the subtype: Maskwood Nexus makes
/// the creature cards p0 owns every type, and a green non-creature spell
/// (Fog) is still not one, so the restricted mana cannot pay it. Plain green
/// does pay it, so mana is not what is missing.
#[test]
fn villainous_hideout_mana_does_not_cast_a_non_creature_spell_even_with_nexus() {
    let p0 = PlayerId::new(0);
    let (mut e, fog) = hideout_green_mana(true, index::FOG);
    assert_eq!(
        crate::casting::can_cast(e.state(), &RegistryLookup, p0, fog),
        Err(crate::casting::CastError::NotEnoughMana),
        "Fog is no Villain, Nexus or not"
    );
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Green, 1);
    e.refresh_offer();
    assert_eq!(
        crate::casting::can_cast(e.state(), &RegistryLookup, p0, fog),
        Ok(()),
        "control: unrestricted green pays Fog's {{G}}"
    );
}

/// p0's main phase with the Hideout and two Maskwood Nexuses' worth of
/// Villains: p0's Ledger Shredder, and p1's Grizzly Bears under p1's own Nexus
/// (Nexus names only creatures "you control"). `{3}` is floating.
fn connive_board() -> (Engine<RegistryLookup>, ObjectId) {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                villainous_hideout(),
                index::MASKWOOD_NEXUS,
                index::LEDGER_SHREDDER,
            ],
        )
        .battlefield(1, &[index::MASKWOOD_NEXUS, index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 3);
    e.refresh_offer();
    let shredder = on_battlefield(&e, p0, index::LEDGER_SHREDDER).expect("Shredder deployed");
    (e, shredder)
}

/// Whether the connive ability (printed ability 2) of the Hideout is offered.
fn connive_offered(e: &Engine<RegistryLookup>) -> bool {
    let hideout = on_battlefield(e, PlayerId::new(0), villainous_hideout()).expect("Hideout");
    matches!(e.pending(), Pending::Priority { legal, .. } if legal.abilities.contains(&(hideout, 2)))
}

/// "Target Villain you control connives": on the menu are p0's Villains and
/// nothing else, so not p0's Hideout and Nexus (permanents, no Villains) and
/// not p1's Grizzly Bears, a Villain under p1's own Nexus but not p0's.
#[test]
fn villainous_hideout_connive_targets_only_a_villain_you_control() {
    let p0 = PlayerId::new(0);
    let (mut e, shredder) = connive_board();
    let bears = on_battlefield(&e, PlayerId::new(1), index::GRIZZLY_BEARS).expect("p1's Bears");
    assert!(
        connive_offered(&e),
        "control: sorcery timing, {{3}} floating"
    );
    activate(&mut e, p0, villainous_hideout(), 2);
    let options = aim_at(&mut e, p0, shredder);
    assert_eq!(
        options,
        vec![shredder],
        "only the Villain p0 controls is a target; p1's Villain ({bears:?}) is not"
    );
}

/// "Activate only as a sorcery", the stack half: a spell waits to resolve, the
/// `{3}` is still floating and the Shredder is still a Villain to aim at.
#[test]
fn villainous_hideout_connive_is_not_offered_with_a_spell_on_the_stack() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                villainous_hideout(),
                index::MASKWOOD_NEXUS,
                index::LEDGER_SHREDDER,
            ],
        )
        .hand(0, &[index::LLANOWAR_ELVES])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let pool = &mut e.dev_state_mut(p0).unwrap().players[0].mana_pool;
    pool.add(ManaColor::Colorless, 3);
    pool.add(ManaColor::Green, 1);
    e.refresh_offer();
    assert!(connive_offered(&e), "control: empty stack, {{3}} floating");
    cast_with_floating(&mut e, p0, index::LLANOWAR_ELVES);
    assert!(!stack_is_empty(&e), "the Elves spell is waiting to resolve");
    assert!(
        e.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless)
            >= 3,
        "the {{3}} is still floating, so mana is not what withholds the ability"
    );
    assert!(
        !connive_offered(&e),
        "\"only as a sorcery\": not while a spell is on the stack"
    );
}

/// "Activate only as a sorcery", the turn half: p1's turn, p0 holding
/// priority with `{3}` floating and a Villain to target.
#[test]
fn villainous_hideout_connive_is_not_offered_on_the_opponents_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut e, _shredder) = connive_board();
    assert!(
        connive_offered(&e),
        "control: the same Hideout, Villain and floating {{3}} offer the ability in p0's own main phase"
    );
    pass_until(&mut e, |e| {
        e.state().turn.active == p1
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 3);
    e.refresh_offer();
    let hideout = on_battlefield(&e, p0, villainous_hideout()).expect("Hideout");
    assert!(
        !is_tapped(&e, hideout),
        "control: the Hideout is untapped, so only the turn withholds the ability"
    );
    assert_eq!(
        e.state().players[0].mana_pool.total(),
        3,
        "control: the {{3}} is floating, so only the turn withholds the ability"
    );
    assert!(
        !connive_offered(&e),
        "\"only as a sorcery\": not on the opponent's turn"
    );
}
