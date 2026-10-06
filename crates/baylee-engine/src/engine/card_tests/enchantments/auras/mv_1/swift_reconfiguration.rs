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

/// The two halves the card's `Coverage::Partial` is about, on a board built
/// so that both of them are visible at once.
///
/// **`with crew 5` is not written.** Five untapped Llanowar Elves stand
/// beside the enchanted permanent — total power exactly 5, which is what the
/// printed crew cost asks for — and the offer the engine makes on that
/// permanent is *exactly* its host's own printed "`{T}`: Add `{G}`", at index
/// 0. An equality and not an emptiness, because an emptiness here would have
/// been wrong rather than weak: a **printed** mana ability is enumerated into
/// `LegalActions::abilities` like any other activated ability —
/// `mana_abilities` is the CR 305.6 land shortcut plus what a continuous
/// effect *granted* — so the Elf under the Aura was never going to offer
/// nothing. The equality keeps what the emptiness was reaching for (the
/// engine is looking at this object) and still fails the moment a second
/// entry appears. The card's own `NOT SUPPORTED` note says why none does: no
/// `CostPart` chooses a set of other creatures and reads a total power off
/// it, so nothing grants a crew ability in the first place. Crew would arrive
/// through `Modifier::GrantActivated`, offered at a `choice::granted_ability`
/// index beside the printed one — offered, which is to say *payable*, since
/// `can_afford` gates the offer. That is what the five untapped Elves are
/// for: with total power exactly 5 standing by, the day crew is written is
/// the day it is affordable, and this assertion is red.
///
/// **A card type does not take its subtypes with it.** `Modifier::RemoveType`
/// clears bits in the projected `types` and nothing subtracts a subtype, so
/// the Elf Druid under the Aura projects as `Artifact — Elf Druid Vehicle`.
/// It is not a creature, which is what every rule this card reaches asks
/// first, but a count of Elves would still find it.
#[test]
#[allow(clippy::too_many_lines)] // both halves of `Coverage::Partial` on one board
fn a_reconfigured_elf_is_a_vehicle_nobody_can_crew_and_keeps_its_creature_types() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(62, forest())
        .battlefield(
            0,
            &[
                plains(),
                llanowar_elves(),
                llanowar_elves(),
                llanowar_elves(),
                llanowar_elves(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[swift_reconfiguration()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elves = mine(&engine, p0, llanowar_elves(), Zone::Battlefield);
    assert_eq!(elves.len(), 6, "one to enchant and five to crew with");
    let (host, crew) = (elves[0], &elves[1..]);

    // Only the Plains pays: the Elves have to stay untapped, because an
    // already-tapped crew would be a second reason for the offer to be absent.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    let card = in_hand(&engine, p0, swift_reconfiguration()).expect("the Aura is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("one Plains pays {W}");
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "expected the Aura's host choice, got {:?}",
            engine.pending()
        )
    };
    assert!(options.contains(&host), "my own Elf is a legal host");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| at_rest(e, p0));

    let now = types(&engine, host);
    assert!(
        now.contains(TypeSet::ARTIFACT) && !now.contains(TypeSet::CREATURE),
        "the Aura resolved and rewrote what the permanent is: {now:?}"
    );

    // The board could pay crew 5 twice over if there were a crew cost to pay.
    assert!(
        crew.iter().all(|id| !is_tapped(&engine, *id)),
        "every creature that would crew it is untapped"
    );
    let total: i16 = crew.iter().map(|id| pt(&engine, *id).0).sum();
    assert_eq!(total, 5, "five untapped 1/1s: total power exactly 5");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let offered: Vec<u32> = legal
        .abilities
        .iter()
        .filter(|(who, _)| *who == host)
        .map(|(_, index)| *index)
        .collect();
    assert_eq!(
        offered,
        [0_u32],
        "the permanent still offers exactly one thing, and it is the Elf's \
         own printed ability at index 0 — `{{T}}: Add {{G}}`, which a printed \
         mana ability is listed under here rather than in `mana_abilities`. \
         `with crew 5` is the card's NOT SUPPORTED clause, so no second entry \
         at a `choice::granted_ability` index stands beside it"
    );
    assert!(
        !types(&engine, host).contains(TypeSet::CREATURE),
        "and with nothing to crew it, it never becomes a creature again"
    );

    // The other half of `Coverage::Partial`: losing the card type leaves the
    // creature types behind.
    let c = engine
        .state()
        .object(host)
        .expect("the host is still on the battlefield")
        .characteristics();
    assert!(
        c.subtypes
            .contains(baylee_core::generated::subtypes::artifact::VEHICLE),
        "Vehicle was added"
    );
    assert!(
        c.subtypes
            .contains(baylee_core::generated::subtypes::creature::ELF)
            && c.subtypes
                .contains(baylee_core::generated::subtypes::creature::DRUID),
        "and Elf and Druid were not taken away with the creature type: no \
         `Modifier` subtracts a subtype. When this fires, the clause has \
         become expressible and the card is no longer Coverage::Partial"
    );
}
