//! `cards/creatures/mv_6/akoum_warrior.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// The front face prints one word — "Trample" — and a chump block is the
/// only scenario that can tell it from nothing at all: CR 702.19b lets the
/// attacker assign its blockers no more than lethal damage and give the rest
/// to the player it is attacking, so a 4/5 held up by a 1/1 Elf puts three
/// through. Without the word the same board deals **nothing** to the player,
/// which is why the life total and not the dead Elf is the assertion (the
/// Elf dies either way, so stopping on its corpse is a stop both worlds
/// reach). Nothing here reads the keyword set: a projected `KeywordSet` says
/// only that the compiled card carries the bit, and it would fail one line
/// ahead of the damage it is supposed to be evidence for.
///
/// The cast says the other half of CR 712.11b on the way in. One card in hand
/// is offered twice — as a land drop *and* as a spell — and the land back is
/// never a cast mode, so a card whose only castable face is the front asks no
/// question and goes straight onto the stack.
#[test]
fn akoum_warrior_tramples_three_points_past_the_elf_that_chumps_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(17, mountain())
        .battlefield(0, &[mountain(); 6])
        .hand(0, &[akoum_warrior()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p0);

    let card = in_hand(&engine, p0, akoum_warrior()).expect("the Warrior is in hand");
    // With the six Mountains already tapped, because `castable` is the list
    // of spells the floating mana pays for and not the list of cards in hand.
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "a main phase hands priority back, got {:?}",
            engine.pending()
        )
    };
    assert!(
        legal.lands.contains(&card),
        "one MDFC in hand is a land drop, because its back face is a land"
    );
    assert!(
        legal.castable.contains(&card),
        "and the same card is a spell, because its front face is a creature"
    );

    engine
        .apply(p0, PlayerAction::CastSpell { card })
        .expect("six Mountains pay for a six-drop");
    assert!(
        matches!(engine.pending(), Pending::Priority { .. }),
        "the land back is played and never cast (CR 712.12), so the front is \
         the only castable face and nothing is asked: {:?}",
        engine.pending()
    );
    pass_until(&mut engine, stack_is_empty);
    let warrior = on_battlefield(&engine, p0, akoum_warrior()).expect("the Minotaur resolved");
    assert_eq!(pt(&engine, warrior), (4, 5), "a 4/5 as printed");

    // Its controller's next turn: summoning sickness has worn off, and the
    // offer of legal attackers is what says so.
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseAttackers { attackers, .. } if attackers.contains(&warrior)
        )
    });
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("the chump stands ready");
    let before = engine.state().players[1].life;
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(warrior, Defender::Player(p1))],
            },
        )
        .expect("a 4/5 that has been out since the turn began may attack");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(elf, warrior)],
            },
        )
        .expect("a 1/1 may block a 4/5");

    pass_until(&mut engine, |e| {
        in_graveyard(e, p1, llanowar_elves()).is_some()
    });
    assert_eq!(
        engine.state().players[1].life,
        before - 3,
        "one lethal point stays on the Elf and the other three go to the \
         player being attacked (CR 702.19b). Without trample a blocked \
         attacker gives the player nothing."
    );
}

/// The back face, Akoum Teeth: "This land enters tapped." and "{T}: Add {R}."
///
/// A land face is *played*, not cast (CR 712.12), and this card prints only
/// one of them, so the land drop resolves straight to face 1 with no face
/// choice to make. Three printed claims follow from that and are struck here:
/// what arrives is a land rather than the 4/5 Minotaur on the other side, it
/// arrives tapped — so it pays for nothing the turn it lands — and once it
/// has untapped it makes one red mana and no more.
///
/// The ability is looked for in `abilities` and not in `mana_abilities`:
/// Akoum Teeth prints no basic land type, so `casting::intrinsic_mana` has
/// nothing to answer with and the printed `{T}: Add {R}` is an ordinary
/// activated ability of the permanent (CR 605.1).
#[test]
fn akoum_teeth_is_played_as_a_land_that_enters_tapped_and_later_taps_for_red() {
    let p0 = PlayerId::new(0);
    let (mut engine, teeth) =
        play_land_face(akoum_warrior(), 1).expect("the back face is a legal land drop");

    assert_eq!(
        engine.state().names.get(
            engine
                .state()
                .object(teeth)
                .expect("the land is on the battlefield")
                .characteristics()
                .name
        ),
        "Akoum Teeth",
        "the land face is the one that arrived"
    );
    assert!(types(&engine, teeth).contains(TypeSet::LAND));
    assert!(
        !types(&engine, teeth).contains(TypeSet::CREATURE),
        "and the Minotaur stayed on the other side of the card"
    );
    assert!(is_tapped(&engine, teeth), "\"This land enters tapped.\"");

    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!(
            "a land drop hands priority back, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "and hands it back to the seat that played it");
    assert!(
        !legal.abilities.iter().any(|(id, _)| *id == teeth),
        "a land that came in tapped cannot pay the tap symbol in its own \
         cost, so it is offered nothing at all on the turn it landed"
    );

    // Its controller's next main phase, with the land untapped — which is the
    // clause that carries the walk past the main phase it is standing in.
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::FirstMain)
            && e.state().turn.active == p0
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
            && !is_tapped(e, teeth)
    });
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        unreachable!("the walk above waited for exactly this")
    };
    assert_eq!(
        legal.abilities,
        vec![(teeth, 0)],
        "once it has untapped the land face's own printed ability is back on \
         offer, and it is the only thing this seat has"
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: teeth,
                ability_index: 0,
            },
        )
        .expect("an untapped Akoum Teeth taps for mana");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "one red mana"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "and nothing else beside it"
    );
    assert!(
        is_tapped(&engine, teeth),
        "paid for by the tap symbol in its own cost"
    );
}
