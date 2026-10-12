//! Actual Underdome payments; synthetic fixtures isolate physical print facts
//! and independently imposed taxes, never replace a registered card's rules.
use super::counter_resources::prepared;
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::{Duration, Filter, Modifier};
use baylee_core::card_metadata::RulePrint;
use baylee_core::generated::index;
use baylee_core::mana::ManaPool;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
mod activations;
mod casting;
mod debts;

fn print_fact(e: &mut Engine<RegistryLookup>, id: ObjectId, is_un: bool) {
    let o = e.state.object_mut(id).unwrap();
    o.physical_rules = Some(std::sync::Arc::new(RulePrint {
        card: o.card.unwrap().index,
        watermarks: vec![None],
        is_un,
    }));
    e.refresh_offer();
}
fn priority(e: &mut Engine<RegistryLookup>, player: PlayerId) {
    pass_until(
        e,
        |e| matches!(e.pending(), Pending::Priority {player:p,..} if *p==player),
    );
}
fn pool(e: &mut Engine<RegistryLookup>, player: PlayerId, ordinary: &[(ManaColor, u32)]) {
    e.state.players[usize::from(player.get())].mana_pool = ManaPool::new();
    for &(color, amount) in ordinary {
        e.state.players[usize::from(player.get())]
            .mana_pool
            .add(color, amount);
    }
    e.refresh_offer();
}
fn restricted(e: &mut Engine<RegistryLookup>, player: PlayerId, color: ManaColor) {
    priority(e, player);
    activate(e, player, index::UNDERDOME, 1);
    e.apply(player, PlayerAction::ChooseColor(color)).unwrap();
}
fn restricted_units(e: &Engine<RegistryLookup>, player: PlayerId) -> u32 {
    e.state.players[usize::from(player.get())]
        .mana_pool
        .restricted()
        .iter()
        .map(|m| u32::from(m.amount))
        .sum()
}
fn impose(e: &mut Engine<RegistryLookup>, source: ObjectId, modifier: Modifier) {
    impose_on(
        e,
        source,
        modifier,
        crate::effects::EffectFilter::Dsl(&Filter::Any),
    );
}
fn impose_on(
    e: &mut Engine<RegistryLookup>,
    source: ObjectId,
    modifier: Modifier,
    filter: crate::effects::EffectFilter,
) {
    // Provenance-only fixture: no implemented registered Un-card imposes this
    // generic increase. The source's real rules and mana ability stay intact.
    let timestamp = e.state.next_timestamp();
    e.state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: Some(source),
        controller: P0,
        origin: crate::effects::EffectOrigin::Resolution,
        layer: modifier.layer(),
        timestamp,
        duration: Duration::WhileSourceOnBattlefield,
        filter,
        modifier,
    });
    e.state.invalidate_projections();
    e.refresh_offer();
}
fn cast_paid(e: &mut Engine<RegistryLookup>, id: ObjectId) {
    e.refresh_offer();
    e.apply(P0, PlayerAction::CastSpell { card: id }).unwrap();
    pass_until(e, stack_is_empty);
    assert_eq!(e.state.object(id).unwrap().zone, Zone::Battlefield);
}
fn reject_cast(e: &mut Engine<RegistryLookup>, id: ObjectId) {
    e.refresh_offer();
    let hash = e.snapshot_hash();
    assert!(e.apply(P0, PlayerAction::CastSpell { card: id }).is_err());
    assert_eq!(e.snapshot_hash(), hash, "unpayable cast is atomic");
    assert_eq!(e.state.object(id).unwrap().zone, Zone::Hand);
}
fn targets(e: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>, players: Vec<PlayerId>) {
    let player = e.pending().asked().unwrap();
    e.apply(player, PlayerAction::ChooseTargets { objects, players })
        .unwrap();
}

mod price_lock;
