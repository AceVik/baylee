//! Helpers for the wave 2 pair D card tests that play Auras: Enchantment
//! Alteration, Gauntlets of Chaos, Infinite Authority and Remove
//! Enchantments.
//!
//! Nothing here knows the cards under test. An Aura is put onto the table
//! the only way a game puts one there -- cast, aimed, resolved -- and the
//! helpers only name what a player could read off the table afterwards:
//! what is attached to what, who owns and who controls it, which zone a card
//! ended in.

#[allow(clippy::wildcard_imports)] // the card tests' own vocabulary
use super::*;

/// Every permanent on the battlefield, in zone order.
pub(super) fn battlefield(engine: &Engine<RegistryLookup>) -> Vec<ObjectId> {
    engine.state().zones.list(ZoneLocation::Battlefield).clone()
}

/// What `object` is attached to, if anything.
pub(super) fn attached_to(engine: &Engine<RegistryLookup>, object: ObjectId) -> Option<ObjectId> {
    engine.state().object(object).and_then(|o| o.attached_to)
}

/// Who controls `object` right now.
#[track_caller]
pub(super) fn controller_of(engine: &Engine<RegistryLookup>, object: ObjectId) -> PlayerId {
    engine
        .state()
        .object(object)
        .expect("the object exists")
        .controller
}

/// Who owns `object`.
#[track_caller]
pub(super) fn owner_of(engine: &Engine<RegistryLookup>, object: ObjectId) -> PlayerId {
    engine
        .state()
        .object(object)
        .expect("the object exists")
        .owner
}

/// How many cards printed as `card` sit in `zone`.
pub(super) fn count_in(
    engine: &Engine<RegistryLookup>,
    zone: ZoneLocation,
    card: CardIndex,
) -> usize {
    engine
        .state()
        .zones
        .list(zone)
        .iter()
        .filter(|id| {
            engine
                .state()
                .object(**id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
        })
        .count()
}

/// Whether `object` is still on the battlefield.
pub(super) fn on_the_table(engine: &Engine<RegistryLookup>, object: ObjectId) -> bool {
    engine
        .state()
        .object(object)
        .is_some_and(|o| o.zone == Zone::Battlefield)
}

/// The zone `object` is in now.
#[track_caller]
pub(super) fn zone_now(engine: &Engine<RegistryLookup>, object: ObjectId) -> Zone {
    engine
        .state()
        .object(object)
        .expect("the object exists")
        .zone
}

/// Casts `aura` from `seat`'s hand off `mana` (put in the pool with the
/// harness' own capability, so the colours are exactly what the card
/// costs), aims it at `host`, lets it resolve, and returns the Aura that is
/// now on the battlefield attached to `host`.
///
/// # Panics
/// When `host` is not among the targets the spell offers, or the spell does
/// not end up attached to it.
#[track_caller]
pub(super) fn cast_aura(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    aura: CardIndex,
    host: ObjectId,
    mana: &[(ManaColor, u32)],
) -> ObjectId {
    let before = battlefield(engine);
    super::legends_kit::float(engine, seat, mana);
    cast_with_floating(engine, seat, aura);
    if let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    {
        assert_eq!(player, seat, "the caster names the host");
        assert!(
            options.contains(&host),
            "the host this scenario wants is a legal one: {options:?}"
        );
        engine
            .apply(
                seat,
                PlayerAction::ChooseTargets {
                    objects: vec![host],
                    players: vec![],
                },
            )
            .expect("the host came out of the offer");
    }
    pass_until(engine, stack_is_empty);
    let arrived: Vec<ObjectId> = battlefield(engine)
        .into_iter()
        .filter(|id| {
            !before.contains(id)
                && attached_to(engine, *id) == Some(host)
                && engine
                    .state()
                    .object(*id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == aura))
        })
        .collect();
    assert_eq!(
        arrived.len(),
        1,
        "exactly one new copy of the Aura is attached to the host: {arrived:?}"
    );
    arrived[0]
}

/// Answers a pending "choose among these permanents" question the way a
/// player would, whichever of the two shapes the engine asks it in (a
/// `ChooseCards` or a `ChooseTargets`), and hands back the menu it offered.
///
/// The menu is returned so that a test can read what was *not* on it: a
/// host the Aura could not legally take is proven by the menu that lacks it,
/// next to the one that has the right answer.
///
/// # Panics
/// When no such question is pending, or `pick` is not on the menu.
#[track_caller]
pub(super) fn choose_among(engine: &mut Engine<RegistryLookup>, pick: ObjectId) -> Vec<ObjectId> {
    match engine.pending().clone() {
        Pending::ChooseCards {
            player, options, ..
        } => {
            assert!(
                options.contains(&pick),
                "the permanent this scenario chooses is on the menu: {options:?}"
            );
            engine
                .apply(
                    player,
                    PlayerAction::ChooseObjects {
                        objects: vec![pick],
                    },
                )
                .expect("the pick came out of the menu");
            options
        }
        Pending::ChooseTargets {
            player, options, ..
        } => {
            assert!(
                options.contains(&pick),
                "the permanent this scenario chooses is on the menu: {options:?}"
            );
            engine
                .apply(
                    player,
                    PlayerAction::ChooseTargets {
                        objects: vec![pick],
                        players: vec![],
                    },
                )
                .expect("the pick came out of the menu");
            options
        }
        other => panic!("a choice among permanents was expected, got {other:?}"),
    }
}

/// Whether the engine is asking a choice among permanents right now.
pub(super) fn asks_for_a_permanent(engine: &Engine<RegistryLookup>) -> bool {
    matches!(
        engine.pending(),
        Pending::ChooseCards { .. } | Pending::ChooseTargets { .. }
    )
}
