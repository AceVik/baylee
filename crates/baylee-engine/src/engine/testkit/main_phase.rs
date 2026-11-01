//! Setup walker for pool-wide probes.

use super::*;

/// Answers mulligans, priorities and empty combat declarations until `seat`
/// holds priority in its own first main phase.
///
/// Tolerant where [`reach_main_phase`] panics: this walker is *setup* for
/// hundreds of unrelated cards, and a board that asks a question it does not
/// know how to answer on the way is a card the caller cannot probe rather
/// than a failure. The distinction is kept honest by a *counted floor* in
/// whichever sweep calls this, which notices when "cannot probe" starts
/// meaning "most of the pool". Every caller owes one.
///
/// Three of those questions are asked by a permanent *on the way in*, and
/// the walker answers them rather than abandoning the board — twenty-three
/// implemented cards, the ten shocklands among them, had no probe at all
/// until it did. What it must not do is answer them the way
/// [`drive_to_rest`] does: the driver is exercising an effect and takes an
/// option wherever one is offered, and this is setup, which wants the card
/// the pool prints. So an optional choice is **declined** — exactly `min`
/// targets, which is none of them for the copy clause on Phantasmal Image
/// and its relatives, because a Cursed Mirror that entered as a copy of
/// something else is not the card the caller came to probe.
///
/// The shockland's question is the one that has no "decline": both answers
/// are legal and one of them puts the land onto the battlefield tapped,
/// where it offers nothing and the probe is pointless. It is answered yes,
/// which is also the answer that leaves the land in the state every other
/// land on this board is already in.
pub fn walk_to_own_main(engine: &mut Engine<RegistryLookup>, seat: PlayerId) -> bool {
    for _ in 0..60 {
        if matches!(engine.state().turn.phase, Phase::FirstMain)
            && engine.state().turn.active == seat
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat)
        {
            return true;
        }
        let refused = match engine.pending().clone() {
            Pending::Mulligan { player, .. } => {
                engine.apply(player, PlayerAction::MulliganKeep).is_err()
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).is_err()
            }
            Pending::ChooseAttackers { player, .. } => engine
                .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                .is_err(),
            Pending::ChooseBlockers { player, .. } => engine
                .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                .is_err(),
            Pending::ChooseTargets {
                player,
                options,
                min,
                ..
            } => engine
                .apply(
                    player,
                    PlayerAction::ChooseTargets {
                        objects: options
                            .into_iter()
                            .take(usize::try_from(min).unwrap())
                            .collect(),
                        players: Vec::new(),
                    },
                )
                .is_err(),
            Pending::ChooseSubtype { player, options } => match options.first().copied() {
                Some(first) => engine
                    .apply(player, PlayerAction::ChooseSubtype(first))
                    .is_err(),
                None => return false,
            },
            Pending::ChooseCardType { player, options } => match options.first().copied() {
                Some(first) => engine
                    .apply(player, PlayerAction::ChooseCardType(first))
                    .is_err(),
                None => return false,
            },
            Pending::ChooseColor { player, options } => match options.first().copied() {
                Some(first) => engine
                    .apply(player, PlayerAction::ChooseColor(first))
                    .is_err(),
                None => return false,
            },
            Pending::YesNo {
                player,
                prompt: crate::choice::YesNoPrompt::PayLifeOrEnterTapped { .. },
                ..
            } => engine.apply(player, PlayerAction::YesNo(true)).is_err(),
            // Decline optional upkeep effects, including Lotus Blossom's
            // counter and Thran Turbine's mana. Setup must reach the main
            // phase without accepting an optional change to the probed card.
            // A tax declined still does what its card says.
            Pending::YesNo {
                player,
                prompt:
                    crate::choice::YesNoPrompt::PayTax { .. }
                    | crate::choice::YesNoPrompt::PayMana { .. }
                    | crate::choice::YesNoPrompt::MayDo,
                ..
            } => engine.apply(player, PlayerAction::YesNo(false)).is_err(),
            _ => return false,
        };
        if refused {
            return false;
        }
    }
    false
}
