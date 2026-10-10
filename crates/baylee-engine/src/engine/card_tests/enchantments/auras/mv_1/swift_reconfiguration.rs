//! `cards/enchantments/auras/mv_1/swift_reconfiguration.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Swift Reconfiguration ({W}, Aura): "Flash. Enchant creature or Vehicle.
/// Enchanted permanent is a Vehicle artifact with crew 5 and it loses all
/// other card types."
///
/// The half that works, played the way the card is played: held through the
/// opponent's turn and flashed onto one of their creatures in their own main
/// phase, which is a cast no sorcery-speed Aura could make (CR 702.8a). The
/// second sentence is struck on the offer rather than on the answer, on all
/// three of its words: the two Elves are on the list, the uncrewed Smuggler's
/// Copter is on it because a *Vehicle* is the other half and not because it
/// is a creature — it is not one — and the Plains standing beside them is on
/// neither, which is the difference between "enchant creature or Vehicle"
/// and "enchant permanent". What the third sentence then does is read off the layer
/// system — layer 4, where the artifact type is added, Vehicle is added as a
/// subtype and every other card type is taken away (CR 613.1d) — and off
/// combat, because an uncrewed Vehicle is not a creature and CR 508.1a only
/// ever declares creatures as attackers.
///
/// The second Elf is the control and carries the whole assertion: both of
/// them start the turn identical and only one is enchanted, so "the engine
/// did not offer it" cannot be a summoning-sick, tapped or otherwise
/// uninteresting board. One is offered as an attacker and the other is not.
#[test]
#[allow(clippy::too_many_lines)] // one scenario, read in order: cast, resolve, combat
fn a_flashed_reconfiguration_makes_an_attacker_a_vehicle_that_cannot_be_declared() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(61, forest())
        .battlefield(0, &[plains(), smugglers_copter()])
        .hand(0, &[swift_reconfiguration()])
        .battlefield(1, &[llanowar_elves(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    let my_plains = on_battlefield(&engine, p0, plains()).expect("one Plains, and it pays {W}");
    let copter = on_battlefield(&engine, p0, smugglers_copter()).expect("an uncrewed Vehicle");
    let elves = mine(&engine, p1, llanowar_elves(), Zone::Battlefield);
    assert_eq!(elves.len(), 2, "two Elves, one enchanted and one not");
    let (enchanted, bystander) = (elves[0], elves[1]);
    assert!(
        types(&engine, enchanted).contains(TypeSet::CREATURE)
            && !types(&engine, enchanted).contains(TypeSet::ARTIFACT),
        "it begins the turn as a plain creature"
    );

    // Their turn, their main phase, and the Aura is still in hand: that is
    // the only window in which flash is the reason it can be cast at all.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain)
            && e.state().turn.active == p1
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    let card = in_hand(&engine, p0, swift_reconfiguration()).expect("held through their turn");
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&card),
        "flash puts the Aura on offer while the other seat is the active player"
    );
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("one Plains pays {W}");

    // "Enchant creature or Vehicle": the Aura picks its host as it is cast,
    // and a land is on neither half of that.
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "an Aura chooses what it enchants as it is cast, got {:?}",
            engine.pending()
        )
    };
    assert!(
        options.contains(&enchanted) && options.contains(&bystander),
        "both creatures are legal hosts: {options:?}"
    );
    assert!(
        options.contains(&copter),
        "and so is the Vehicle, which is the half of the line no creature can \
         stand for — it is on the list because it is a Vehicle and not \
         because it is a creature, which it is not: {options:?}"
    );
    assert!(
        !options.contains(&my_plains),
        "while the land is on neither half, which is the word the printed \
         line does not say: {options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![enchanted],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, swift_reconfiguration()).is_some()
    });

    let aura = on_battlefield(&engine, p0, swift_reconfiguration()).expect("the Aura resolved");
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(enchanted),
        "and arrived attached to the creature it targeted"
    );
    let now = types(&engine, enchanted);
    assert!(
        now.contains(TypeSet::ARTIFACT) && !now.contains(TypeSet::CREATURE),
        "it is an artifact and has lost every other card type: {now:?}"
    );
    assert!(
        engine
            .state()
            .object(enchanted)
            .expect("the host is still on the battlefield")
            .characteristics()
            .subtypes
            .contains(baylee_core::generated::subtypes::artifact::VEHICLE),
        "and a Vehicle"
    );

    // CR 508.1a declares creatures, so an uncrewed Vehicle is not on the list
    // — while the Elf beside it, identical in every other way, is.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    assert_eq!(
        engine.state().object(aura).and_then(|o| o.attached_to),
        Some(enchanted),
        "and it is still attached after the state-based actions have run, \
         with its host no longer a creature (CR 704.5m)"
    );
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until only stops on the attacker declaration")
    };
    assert!(
        attackers.contains(&bystander),
        "the untouched Elf attacks, so the board itself is not the reason"
    );
    assert!(
        !attackers.contains(&enchanted),
        "the enchanted one is no creature and may not be declared: {attackers:?}"
    );
}

/// The type rewrite removes creature subtypes; the granted crew ability restores creaturehood.
#[test]
fn a_reconfigured_elf_can_be_crewed_and_loses_its_old_creature_types() {
    use crate::engine::card_tests::legends_kit as kit;
    use baylee_core::generated::subtypes;
    let mut e = kit::game(
        979,
        &[llanowar_elves(); 6],
        &[],
        &[swift_reconfiguration()],
        &[],
    );
    let elves = mine(&e, kit::P0, llanowar_elves(), Zone::Battlefield);
    let (host, crew) = (elves[0], &elves[1..]);
    kit::float(&mut e, kit::P0, &[(ManaColor::White, 1)]);
    kit::cast_at(&mut e, kit::P0, swift_reconfiguration(), &[host], &[]);
    assert_eq!(types(&e, host), TypeSet::ARTIFACT);
    let c = e.state.object(host).unwrap().characteristics();
    assert!(c.subtypes.contains(subtypes::artifact::VEHICLE));
    assert!(!c.subtypes.contains(subtypes::creature::ELF));
    assert!(!c.subtypes.contains(subtypes::creature::DRUID));
    let ability = crate::choice::granted_ability(0);
    assert!(priority_offer(&e).abilities.contains(&(host, ability)));
    e.apply(
        kit::P0,
        PlayerAction::ActivateAbility {
            source: host,
            ability_index: ability,
        },
    )
    .unwrap();
    assert!(
        e.apply(
            kit::P0,
            PlayerAction::ChooseObjects {
                objects: crew[..4].to_vec()
            }
        )
        .is_err()
    );
    assert!(crew.iter().all(|id| !is_tapped(&e, *id)));
    e.apply(
        kit::P0,
        PlayerAction::ChooseObjects {
            objects: crew.to_vec(),
        },
    )
    .unwrap();
    assert!(crew.iter().all(|id| is_tapped(&e, *id)));
    pass_until(&mut e, stack_is_empty);
    assert_eq!(types(&e, host), TypeSet::ARTIFACT.union(TypeSet::CREATURE));
    reach_their_main_phase(&mut e, kit::P1);
    assert_eq!(types(&e, host), TypeSet::ARTIFACT);
}
