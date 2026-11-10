//! `cards/enchantments/mv_4/orcish_oriflamme.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn orcish_oriflamme() -> CardIndex {
    card_index("0b16a650-68b0-44dc-a9e1-15b7966e0b18")
}

/// Walks to the question where `seat` names attackers with `creature` among
/// the options, and declares it the attacker at the other seat.
#[track_caller]
fn attack_with(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    creature: ObjectId,
    at: PlayerId,
) {
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseAttackers { player, attackers, .. }
                if *player == seat && attackers.contains(&creature)
        )
    });
    engine
        .apply(
            seat,
            PlayerAction::DeclareAttackers {
                attackers: vec![(creature, Defender::Player(at))],
            },
        )
        .expect("a creature that has been out since the turn began may attack");
}

/// Orcish Oriflamme: "Attacking creatures you control get +1/+0." The Elf
/// that attacks is 2/1; the Ogre that stays home is as printed, and once the
/// combat is over the Elf is a 1/1 again: the bonus is for the attack, not
/// for the creature.
#[test]
fn oriflamme_pumps_an_attacker_for_the_attack_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[orcish_oriflamme(), llanowar_elves(), gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the attacker");
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("the one that stays home");
    assert_eq!(pt(&engine, elf), (1, 1), "at home: as printed");

    attack_with(&mut engine, p0, elf, p1);
    assert_eq!(pt(&engine, elf), (2, 1), "attacking: +1/+0");
    assert_eq!(
        pt(&engine, ogre),
        (2, 2),
        "a creature we control that is not attacking: untouched"
    );

    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "combat is over, the Elf no longer attacks: the bonus is gone"
    );
}

/// "Attacking creatures you control": a blocker is not an attacker, and the
/// defending player's creature is not ours. Their Ogre, blocking our pumped
/// Elf, stays 2/2.
#[test]
fn oriflamme_does_not_pump_the_blocker() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[orcish_oriflamme(), llanowar_elves()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the attacker");
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("the blocker");

    attack_with(&mut engine, p0, elf, p1);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseBlockers { player, .. } if *player == p1),
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(ogre, elf)],
            },
        )
        .expect("the Ogre may block the Elf");
    assert_eq!(pt(&engine, elf), (2, 1), "the attacker keeps the bonus");
    assert_eq!(
        pt(&engine, ogre),
        (2, 2),
        "the blocker is neither attacking nor ours: as printed"
    );
}

/// The `you control` half, which a board with the enchantment on the
/// attacker's side cannot see: an Oriflamme under p0 does nothing for the
/// Elf p1 attacks with, while the identical attack under an Oriflamme p1
/// controls is pumped. Without the control the 1/1 could just be a harness
/// that never pumps anyone on turn 2.
#[test]
fn oriflamme_does_not_pump_the_opponents_attackers() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[orcish_oriflamme()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("the opponent's Elf");
    attack_with(&mut engine, p1, their_elf, p0);
    assert_eq!(
        pt(&engine, their_elf),
        (1, 1),
        "an attacker we do not control gets nothing from our Oriflamme"
    );

    let mut own = Duel::new(SEED, forest())
        .battlefield(0, &[quiet_creature()])
        .battlefield(1, &[orcish_oriflamme(), llanowar_elves()])
        .start();
    keep_mulligans(&mut own);
    let elf = on_battlefield(&own, p1, llanowar_elves()).expect("the Elf");
    attack_with(&mut own, p1, elf, p0);
    assert_eq!(
        pt(&own, elf),
        (2, 1),
        "control: under its own controller's Oriflamme the same attack is pumped"
    );
}
