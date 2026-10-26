//! The arrival-control cases: blinks and returns played from a stolen creature.

use super::*;
use crate::engine::synthetic::SyntheticLookup;
use baylee_cards_dsl::Filter;

fn me() -> PlayerId {
    PlayerId::new(0)
}

fn them() -> PlayerId {
    PlayerId::new(1)
}

/// A two-seat game with a creature seat 1 owns on the battlefield.
fn theirs(seed: u64) -> (GameState, ObjectId) {
    let card = crate::engine::synthetic::creature(900_123, "Creature", 2, 2, &[]);
    let state = GameState::from_preset(
        &crate::engine::synthetic::preset_both(seed, &[], &[900_123]),
        &SyntheticLookup::new(vec![card]),
    )
    .expect("a two-seat game with a real card, not a token");
    let it = state.zones.list(ZoneLocation::Battlefield)[0];
    (state, it)
}

/// A creature seat 1 owns and seat 0 has stolen.
fn stolen() -> (GameState, ObjectId) {
    let (mut state, it) = theirs(610);
    crate::resolve::gain_control(&mut state, &[(it, me())]);
    let obj = state.object(it).expect("still there");
    assert_eq!(
        (obj.owner, obj.controller, obj.base_controller),
        (them(), me(), them()),
        "the steal is a layer-2 effect over seat 1's own default"
    );
    (state, it)
}

/// Seat 0 blinks `it`: an ability of seat 0's, resolving.
fn blink(state: &mut GameState, it: ObjectId, owner_control: bool) {
    let effect = Effect::Blink {
        target: TargetSpec::Object(&Filter::CREATURE),
        owner_control,
    };
    resolve(state, ObjectId::NO_SOURCE, &[it], effect);
}

/// An ability of seat 0's with `source` and `targets`, resolving.
fn resolve(state: &mut GameState, source: ObjectId, targets: &[ObjectId], effect: Effect) {
    resolve_from(state, source, ObjectId::NO_SOURCE, targets, effect);
}

/// [`resolve`], for the ability `on_stack` stands for.
fn resolve_from(
    state: &mut GameState,
    source: ObjectId,
    on_stack: ObjectId,
    targets: &[ObjectId],
    effect: Effect,
) {
    let mut res = Resolution {
        source,
        on_stack,
        controller: me(),
        effects: vec![effect],
        pc: 0,
        targets: SmallVec::from_slice(targets),
        second_targets: SmallVec::new(),
        x: None,
        chosen_player: None,
        target_lki: None,
        subject: crate::resolve::SubjectContext::default(),
        text: crate::text_changes::TextChangeMap::IDENTITY,
        event_mana: None,
        retarget_left: None,
        loot: None,
        draw_watch: None,
        target_players: baylee_core::ids::SeatSet::new(),
        event_object: None,
        awaiting: None,
        targeted: true,
        mana_ability: false,
        countered_source: None,
        this_way_from: None,
        coin_stake: None,
    };
    assert!(matches!(run(state, &mut res), Flow::Complete));
    state.refresh_characteristics();
}

/// "…then return that card to the battlefield under your control"
/// (Restoration Angel): the new object enters under seat 0's control
/// (CR 110.2a, CR 400.7) and stays there with no control effect holding
/// it, while seat 1 still owns it (CR 108.3).
#[test]
fn under_your_control_a_stolen_creature_comes_back_yours_and_still_theirs() {
    let (mut state, it) = stolen();
    blink(&mut state, it, false);
    let obj = state.object(it).expect("the same arena handle");
    assert_eq!(obj.zone, crate::zone::Zone::Battlefield, "it came back");
    assert_eq!(
        (obj.controller, obj.base_controller),
        (me(), me()),
        "entered under the blinking player's control, by default and not by an effect"
    );
    assert_eq!(obj.owner, them(), "ownership never changes");

    // The steal effect named the old object (CR 400.7), and ending it
    // hands nothing back: seat 0's control is the new object's own.
    state
        .effects
        .remove_where(|fx| fx.modifier == baylee_cards_dsl::Modifier::GainControl);
    state.refresh_characteristics();
    assert_eq!(state.object(it).map(|o| o.controller), Some(me()));

    // "You own" still reads the owner.
    let obj = state.object(it).expect("still there");
    assert!(!eval::matches(&Filter::OwnedByYou, &state, obj, me(), it));
    assert!(eval::matches(&Filter::OwnedByYou, &state, obj, them(), it));

    // And it dies into its owner's graveyard (CR 400.3).
    sba::destroy(&mut state, it);
    assert_eq!(state.zones.list(ZoneLocation::Graveyard(them()))[..], [it]);
    assert!(state.zones.list(ZoneLocation::Graveyard(me())).is_empty());
}

/// "…under its owner's control" (Ephemerate): the other sentence, on the
/// same board, sends the creature home.
#[test]
fn under_its_owners_control_a_stolen_creature_goes_home() {
    let (mut state, it) = stolen();
    blink(&mut state, it, true);
    let obj = state.object(it).expect("the same arena handle");
    assert_eq!(obj.zone, crate::zone::Zone::Battlefield, "it came back");
    assert_eq!(
        (obj.owner, obj.controller, obj.base_controller),
        (them(), them(), them())
    );
}

/// A creature kept this way leaves the game with its owner (CR 800.4a:
/// "all objects … owned by that player leave the game").
#[test]
fn a_creature_kept_by_a_blink_leaves_the_game_with_its_owner() {
    let (mut state, it) = stolen();
    blink(&mut state, it, false);
    assert_eq!(state.object(it).map(|o| o.controller), Some(me()));
    crate::sba::eliminate_player(&mut state, them(), crate::event::LossReason::Conceded);
    assert!(state.object(it).is_none());
    assert!(!state.zones.list(ZoneLocation::Battlefield).contains(&it));
}

/// Nothing is put onto the battlefield under the control of a player
/// who has left (CR 800.4b): a blink "under your control" resolving for
/// one exiles its target and leaves it in exile. Under its owner's it
/// comes back, since its owner is still in the game.
#[test]
fn a_blink_under_your_control_returns_nothing_to_a_player_who_has_left() {
    for (owner_control, back) in [(false, false), (true, true)] {
        let (mut state, it) = theirs(611);
        crate::sba::eliminate_player(&mut state, me(), crate::event::LossReason::Conceded);
        blink(&mut state, it, owner_control);
        let obj = state.object(it).expect("seat 1's own card");
        let want = if back {
            crate::zone::Zone::Battlefield
        } else {
            crate::zone::Zone::Exile
        };
        assert_eq!(obj.zone, want, "owner_control: {owner_control}");
    }
}

/// Seat 0's ability of `it`'s own exiles `it` and returns it
/// (`ExileSelfReturnAsFace`): Fable of the Mirror-Breaker III, Sheoldred's
/// `{4}{B}`, a transform's stand-in.
fn self_return(state: &mut GameState, it: ObjectId, owner_control: bool) {
    let effect = Effect::ExileSelfReturnAsFace {
        face: 0,
        owner_control,
    };
    resolve(state, it, &[], effect);
}

#[test]
fn a_stale_or_phased_source_cannot_blink_a_new_incarnation() {
    for phased in [false, true] {
        let (mut state, it) = stolen();
        let version = state.object(it).unwrap().version;
        let name = state.names.intern("Return ability");
        let ability =
            state.create_bare(me(), ObjectKind::AbilityOnStack, name, ZoneLocation::Stack);
        state
            .object_mut(ability)
            .unwrap()
            .riders
            .push(crate::object::Rider::AbilitySourceVersion(version));
        if phased {
            state
                .object_mut(it)
                .unwrap()
                .status
                .insert(crate::object::Status::PHASED_OUT);
        } else {
            state
                .move_object(
                    it,
                    ZoneLocation::Exile(them()),
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .unwrap();
            state
                .move_object(
                    it,
                    ZoneLocation::Battlefield,
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .unwrap();
        }
        state.refresh_characteristics();
        let identity = state.source_identity(it);
        let before = state.journal.len();
        resolve_from(
            &mut state,
            it,
            ability,
            &[],
            Effect::ExileSelfReturnAsFace {
                face: 0,
                owner_control: false,
            },
        );
        assert_eq!(state.source_identity(it), identity);
        assert_eq!(state.journal.len(), before);
    }
}

/// "Exile this Saga, then return it to the battlefield transformed under
/// your control" on a permanent seat 0 has stolen: the new object enters
/// under seat 0's control by default, not an effect's (CR 110.2a,
/// CR 400.7), and stays with seat 0 once the steal is gone, while seat 1
/// still owns it (CR 108.3). The effect returned every card under its
/// owner's control, so the thief's Fable went home at chapter III.
#[test]
fn a_stolen_permanent_returning_itself_under_your_control_stays_with_the_thief() {
    let (mut state, it) = stolen();
    self_return(&mut state, it, false);
    let obj = state.object(it).expect("the same arena handle");
    assert_eq!(obj.zone, crate::zone::Zone::Battlefield, "it came back");
    assert_eq!(
        (obj.owner, obj.controller, obj.base_controller),
        (them(), me(), me()),
        "under the controller of the ability, by default, still seat 1's card"
    );
    state
        .effects
        .remove_where(|fx| fx.modifier == baylee_cards_dsl::Modifier::GainControl);
    state.refresh_characteristics();
    assert_eq!(state.object(it).map(|o| o.controller), Some(me()));
}

/// "…then return it to the battlefield transformed under its owner's
/// control" (Sheoldred, the Ojers): the other sentence, on the same
/// board, sends the stolen permanent home.
#[test]
fn a_stolen_permanent_returning_itself_under_its_owners_control_goes_home() {
    let (mut state, it) = stolen();
    self_return(&mut state, it, true);
    let obj = state.object(it).expect("the same arena handle");
    assert_eq!(obj.zone, crate::zone::Zone::Battlefield, "it came back");
    assert_eq!(
        (obj.owner, obj.controller, obj.base_controller),
        (them(), them(), them())
    );
}

/// A self-return "under your control" for a player who has left leaves
/// the card in exile (CR 800.4b); under its owner's it comes back.
#[test]
fn a_self_return_under_your_control_returns_nothing_to_a_player_who_has_left() {
    for (owner_control, back) in [(false, false), (true, true)] {
        let (mut state, it) = theirs(612);
        crate::sba::eliminate_player(&mut state, me(), crate::event::LossReason::Conceded);
        self_return(&mut state, it, owner_control);
        let obj = state.object(it).expect("seat 1's own card");
        let want = if back {
            crate::zone::Zone::Battlefield
        } else {
            crate::zone::Zone::Exile
        };
        assert_eq!(obj.zone, want, "owner_control: {owner_control}");
    }
}

/// Restoration Angel keeps the stolen creature: seat 0 now controls it
/// by default, and seat 1 still owns it.
fn kept() -> (GameState, ObjectId) {
    let (mut state, it) = stolen();
    blink(&mut state, it, false);
    let obj = state.object(it).expect("back");
    assert_eq!((obj.owner, obj.base_controller), (them(), me()));
    (state, it)
}

/// A permanent of seat 0's that `ExileLinked` names as the host.
fn host(state: &mut GameState) -> ObjectId {
    let name = state.names.intern("Host");
    let card = state
        .battlefield_seen()
        .find_map(|id| state.object(id).and_then(|o| o.card))
        .expect("physical fixture card");
    let host = state.create_bare(me(), ObjectKind::Permanent, name, ZoneLocation::Battlefield);
    state.object_mut(host).unwrap().card = Some(card);
    host
}

/// Endless Sands and Safe Haven: "Return each creature card exiled with
/// this land to the battlefield under its owner's control." The kept
/// creature goes home (CR 610.3c), not back to the default a blink gave
/// it.
#[test]
fn a_linked_exile_returns_under_its_owners_control() {
    let (mut state, it) = kept();
    let host = host(&mut state);
    let exile = Effect::exile_linked(TargetSpec::Object(&Filter::CREATURE));
    resolve(&mut state, host, &[it], exile);
    assert_eq!(
        state.object(it).map(|o| o.zone),
        Some(crate::zone::Zone::Exile)
    );
    resolve(&mut state, host, &[], Effect::ReturnLinkedToBattlefield);
    let obj = state.object(it).expect("back");
    assert_eq!(obj.zone, crate::zone::Zone::Battlefield);
    assert_eq!(
        (obj.owner, obj.controller, obj.base_controller),
        (them(), them(), them())
    );
}

/// Palace Jailer: "exile target creature an opponent controls until an
/// opponent becomes the monarch". The card returns when the "until"
/// event happens, under its owner's control (CR 610.3c).
#[test]
fn a_monarch_linked_exile_returns_under_its_owners_control() {
    let (mut state, it) = kept();
    let host = host(&mut state);
    resolve(&mut state, host, &[it], until_crowned());
    state.set_monarch(them());
    state.refresh_characteristics();
    let obj = state.object(it).expect("back");
    assert_eq!(obj.zone, crate::zone::Zone::Battlefield);
    assert_eq!(
        (obj.owner, obj.controller, obj.base_controller),
        (them(), them(), them())
    );
}

// --- "Exile … until …" (CR 610.3) ---------------------------------

/// Werefox Bodyguard's exile: "until this creature leaves the
/// battlefield".
fn until_it_leaves() -> Effect {
    Effect::exile_until(
        TargetSpec::Object(&Filter::CREATURE),
        baylee_cards_dsl::ExileUntil::SourceLeavesBattlefield,
    )
}

/// Palace Jailer's exile: "until an opponent becomes the monarch".
fn until_crowned() -> Effect {
    Effect::exile_until(
        TargetSpec::Object(&Filter::CREATURE),
        baylee_cards_dsl::ExileUntil::OpponentBecomesMonarch,
    )
}

fn zone(state: &GameState, id: ObjectId) -> Option<crate::zone::Zone> {
    state.object(id).map(|o| o.zone)
}

/// Whether `id` still carries a link to anything.
fn linked(state: &GameState, id: ObjectId) -> bool {
    state.object(id).is_some_and(|o| {
        o.riders
            .iter()
            .any(|r| matches!(r, crate::object::Rider::Linked { .. }))
    })
}

/// A second creature seat 1 owns, beside the one `theirs` made.
fn another_of_theirs(state: &mut GameState) -> ObjectId {
    let name = state.names.intern("Other Creature");
    let it = state.create_bare(
        them(),
        ObjectKind::Permanent,
        name,
        ZoneLocation::Battlefield,
    );
    let obj = state.object_mut(it).expect("fresh");
    let mut base = (*obj.base).clone();
    base.types = baylee_core::types::TypeSet::CREATURE;
    obj.base = std::sync::Arc::new(base);
    state.invalidate_projections();
    it
}

/// The card comes back as the host leaves, inside the move that takes
/// the host away: nothing is resolved, refreshed or checked in between
/// (CR 610.3). It comes back under its owner's control (CR 610.3c),
/// here not the default the blink that kept it gave it, and linked to
/// nothing.
#[test]
fn an_exile_until_its_host_leaves_ends_as_the_host_leaves() {
    let (mut state, it) = kept();
    let host = host(&mut state);
    resolve(&mut state, host, &[it], until_it_leaves());
    assert_eq!(zone(&state, it), Some(crate::zone::Zone::Exile));

    sba::destroy(&mut state, host);
    assert_eq!(zone(&state, host), Some(crate::zone::Zone::Graveyard));
    let obj = state.object(it).expect("back");
    assert_eq!(obj.zone, crate::zone::Zone::Battlefield);
    assert_eq!(
        (obj.owner, obj.controller, obj.base_controller),
        (them(), them(), them())
    );
    assert!(!linked(&state, it));
}

/// A host that is blinked leaves the battlefield too, and what comes
/// back is a new object that holds nothing (CR 400.7).
#[test]
fn a_blinked_host_lets_go_of_what_it_held() {
    let (mut state, it) = theirs(613);
    let host = host(&mut state);
    resolve(&mut state, host, &[it], until_it_leaves());
    let blink = Effect::blink_to_owner(TargetSpec::Object(&Filter::Any));
    resolve(&mut state, ObjectId::NO_SOURCE, &[host], blink);
    assert_eq!(zone(&state, host), Some(crate::zone::Zone::Battlefield));
    assert_eq!(zone(&state, it), Some(crate::zone::Zone::Battlefield));
    assert!(!linked(&state, it));
}

/// A host that leaves the game with its owner leaves the battlefield
/// without a move (CR 800.4a), and lets go all the same.
#[test]
fn a_host_that_leaves_the_game_lets_go_of_what_it_held() {
    let (mut state, it) = theirs(614);
    let host = host(&mut state);
    resolve(&mut state, host, &[it], until_it_leaves());
    crate::sba::eliminate_player(&mut state, me(), crate::event::LossReason::Conceded);
    assert!(state.object(host).is_none(), "gone with seat 0");
    let obj = state.object(it).expect("seat 1's own creature");
    assert_eq!(obj.zone, crate::zone::Zone::Battlefield);
    assert_eq!((obj.owner, obj.controller), (them(), them()));
}

/// If the host has left before the exile would happen, the card does
/// not move (CR 610.3a, 610.3b).
#[test]
fn an_exile_until_the_host_leaves_does_nothing_once_it_has_left() {
    let (mut state, it) = theirs(615);
    let version = state.object(it).expect("there").version;
    let host = host(&mut state);
    sba::destroy(&mut state, host);
    resolve(&mut state, host, &[it], until_it_leaves());
    assert_eq!(zone(&state, it), Some(crate::zone::Zone::Battlefield));
    assert_eq!(state.object(it).map(|o| o.version), Some(version));
}

/// Nor when the host left while the ability waited and came back as a
/// new object (CR 400.7), which the ability on the stack wrote down as
/// the host went (`source_power_lki`).
#[test]
fn an_exile_until_the_host_leaves_does_nothing_once_it_has_been_blinked() {
    let (mut state, it) = theirs(618);
    let version = state.object(it).expect("there").version;
    let host = host(&mut state);
    let name = state.names.intern("Enters Trigger");
    let ability = state.create_bare(me(), ObjectKind::AbilityOnStack, name, ZoneLocation::Stack);
    state.object_mut(ability).expect("fresh").ability = Some(crate::object::AbilityLoc {
        card: None,
        index: 0,
        source: host,
    });
    let blink = Effect::blink_to_owner(TargetSpec::Object(&Filter::Any));
    resolve(&mut state, ObjectId::NO_SOURCE, &[host], blink);
    assert_eq!(zone(&state, host), Some(crate::zone::Zone::Battlefield));
    resolve_from(&mut state, host, ability, &[it], until_it_leaves());
    assert_eq!(
        zone(&state, it),
        Some(crate::zone::Zone::Battlefield),
        "the host on the battlefield is not the object whose ability this is"
    );
    assert_eq!(state.object(it).map(|o| o.version), Some(version));

    // And the same ability, whose source never left, does exile.
    state
        .object_mut(ability)
        .expect("still there")
        .source_power_lki = None;
    resolve_from(&mut state, host, ability, &[it], until_it_leaves());
    assert_eq!(zone(&state, it), Some(crate::zone::Zone::Exile));
}

/// The monarch ends the exile that waits for an opponent of the player
/// who exiled, and no other: not an exile with no end of its own
/// (Skyclave Apparition), and not while the new monarch is that player.
#[test]
fn the_monarch_ends_only_the_exile_that_waits_for_it() {
    let (mut state, jailed) = theirs(616);
    let held = another_of_theirs(&mut state);
    let jailer = host(&mut state);
    let apparition = host(&mut state);
    resolve(&mut state, jailer, &[jailed], until_crowned());
    let for_good = Effect::exile_linked(TargetSpec::Object(&Filter::CREATURE));
    resolve(&mut state, apparition, &[held], for_good);

    state.set_monarch(me());
    assert_eq!(zone(&state, jailed), Some(crate::zone::Zone::Exile));
    state.set_monarch(them());
    assert_eq!(zone(&state, jailed), Some(crate::zone::Zone::Battlefield));
    assert_eq!(
        zone(&state, held),
        Some(crate::zone::Zone::Exile),
        "an exile with no \"until\" is not ended by a crown"
    );
}

/// "An opponent" is an opponent of the player who controlled the
/// exiling ability, whoever controls the Jailer by the time the crown
/// moves. Seat 1 takes the Jailer and then the crown: seat 1 is still
/// an opponent of seat 0, who exiled, and the card comes back.
#[test]
fn a_stolen_jailer_still_waits_for_an_opponent_of_the_player_who_exiled() {
    let (mut state, jailed) = theirs(617);
    let jailer = host(&mut state);
    resolve(&mut state, jailer, &[jailed], until_crowned());
    state.set_monarch(me());
    crate::resolve::gain_control(&mut state, &[(jailer, them())]);
    assert_eq!(state.object(jailer).map(|o| o.controller), Some(them()));
    state.set_monarch(them());
    assert_eq!(zone(&state, jailed), Some(crate::zone::Zone::Battlefield));
}

/// Takes `it` out of exile the way a cast does (onto the stack, then the
/// battlefield as it resolves) and exiles it again with no link, the way
/// Swords to Plowshares does.
fn cast_and_exiled_again(state: &mut GameState, it: ObjectId) {
    for to in [
        ZoneLocation::Stack,
        ZoneLocation::Battlefield,
        ZoneLocation::Exile(them()),
    ] {
        state
            .move_object(it, to, ZonePosition::Top, Cause::Effect)
            .expect("the card is there");
    }
}

/// A card that leaves exile is a new object with no relation to the exile
/// it left (CR 400.7), so once it is back in exile some other way it is
/// not "exiled with" the host that exiled it first, and Safe Haven's
/// return leaves it where it is.
///
/// The link rode along with every move, so the card that was cast out of
/// Safe Haven's exile and later hit by Swords to Plowshares came back when
/// Safe Haven was sacrificed.
#[test]
fn a_card_that_left_exile_is_no_longer_exiled_with_its_old_host() {
    let (mut state, it) = theirs(619);
    let haven = host(&mut state);
    resolve(
        &mut state,
        haven,
        &[it],
        Effect::exile_linked(TargetSpec::Object(&Filter::CREATURE)),
    );
    assert!(linked(&state, it), "exiled with Safe Haven");
    cast_and_exiled_again(&mut state, it);
    resolve(&mut state, haven, &[], Effect::ReturnLinkedToBattlefield);
    assert_eq!(zone(&state, it), Some(crate::zone::Zone::Exile));
    assert!(!linked(&state, it), "a new object, exiled by nothing");
}

/// The same for an exile that lasted until its host leaves the
/// battlefield: the Bodyguard leaving lets go of the card it exiled, and
/// the card in exile now is not that card.
#[test]
fn a_card_that_left_exile_does_not_come_back_as_its_old_host_leaves() {
    let (mut state, it) = theirs(620);
    let bodyguard = host(&mut state);
    resolve(&mut state, bodyguard, &[it], until_it_leaves());
    cast_and_exiled_again(&mut state, it);
    sba::destroy(&mut state, bodyguard);
    assert_eq!(zone(&state, it), Some(crate::zone::Zone::Exile));
    assert!(!linked(&state, it));
}

/// Every rider that says what a card is *in exile* ends as the card leaves
/// exile (CR 400.7): exiled with a host, on an adventure (CR 715.3d, "for
/// as long as that card remains exiled"), castable from exile by a
/// player, suspended, rebounding, foretold, plotted. A move from exile to
/// exile is no leaving, and keeps them all.
#[test]
fn what_a_card_was_in_exile_ends_as_it_leaves_exile() {
    use crate::object::Rider;
    let (mut state, it) = theirs(621);
    let host = host(&mut state);
    let in_exile = [
        Rider::Linked { host, until: None },
        Rider::ExiledWith { host, version: 0 },
        Rider::Adventure(me()),
        Rider::PlayableFromExileFor(me()),
        Rider::Suspend,
        Rider::Rebound,
        Rider::Foretold,
        Rider::Plotted,
    ];
    let riders = |state: &GameState| state.object(it).expect("there").riders.clone();
    for to in [ZoneLocation::Exile(them()), ZoneLocation::Exile(them())] {
        state
            .move_object(it, to, ZonePosition::Top, Cause::Effect)
            .expect("the card is there");
        if riders(&state).is_empty() {
            state
                .object_mut(it)
                .expect("in exile")
                .riders
                .extend(in_exile);
        }
    }
    assert_eq!(riders(&state)[..], in_exile, "exile to exile keeps them");
    state
        .move_object(
            it,
            ZoneLocation::Hand(them()),
            ZonePosition::Top,
            Cause::Effect,
        )
        .expect("the card is there");
    assert!(riders(&state).is_empty(), "{:?}", riders(&state));
}

/// Coiling Oracle: "Reveal the top card of your library. If it's a land
/// card, put it onto the battlefield." No control is named, so the land
/// enters under the player the effect told to put it there (CR 110.2a)
/// — whatever default the card carried from its last time on the
/// battlefield, here seat 1's.
#[test]
fn a_revealed_land_enters_under_the_player_who_puts_it_there() {
    let (mut state, _) = theirs(612);
    let name = state.names.intern("Land");
    let land = state.create_bare(me(), ObjectKind::Card, name, ZoneLocation::Library(me()));
    state
        .object_mut(land)
        .expect("fresh")
        .set_controller(them());
    let reveal = Effect::RevealTopAndSort {
        filter: &Filter::Any,
        matched: baylee_cards_dsl::effect::SearchDest::Battlefield,
        otherwise: baylee_cards_dsl::effect::SearchDest::Hand,
    };
    resolve(&mut state, ObjectId::NO_SOURCE, &[], reveal);
    let obj = state.object(land).expect("put onto the battlefield");
    assert_eq!(obj.zone, crate::zone::Zone::Battlefield);
    assert_eq!(
        (obj.owner, obj.controller, obj.base_controller),
        (me(), me(), me())
    );
}
