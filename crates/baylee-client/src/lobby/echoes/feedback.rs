//! Authoritative revision differences become bounded, non-interactive visual feedback.
use super::{LobbyState, Page, api};
use baylee_client_core::echoes::feedback::{self, Trace};
use bevy::prelude::*;
pub(super) use feedback::Anchor;
use std::collections::BTreeMap;

#[derive(Component)]
pub(super) struct Place {
    run: uuid::Uuid,
    revision: u64,
    key: Anchor,
}

impl Place {
    pub(super) fn new(run: &api::Run, key: Anchor) -> Self {
        Self {
            run: run.id,
            revision: run.revision,
            key,
        }
    }
}

#[derive(Component)]
struct Transient;

#[derive(Default)]
struct Retained {
    trace: Trace,
    income: feedback::Income,
    run: Option<uuid::Uuid>,
    positions: BTreeMap<Anchor, Vec2>,
}

pub(super) fn install(app: &mut App) {
    crate::towerui::motion::install(app);
    app.add_systems(PostUpdate, observe.after(bevy::ui::UiSystems::Layout));
}

#[allow(clippy::too_many_arguments)] // Read-only state/geometry; no commands to the authority.
fn observe(
    mut commands: Commands,
    state: Res<LobbyState>,
    fonts: Option<Res<crate::hud::UiFonts>>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    places: Query<(&Place, Ref<ComputedNode>, Ref<UiGlobalTransform>)>,
    effects: Query<Entity, With<Transient>>,
    mut sounds: Option<MessageWriter<crate::sound::tower::TowerSound>>,
    mut previous: Local<Retained>,
) {
    let Some(run) = state.echoes.model.run() else {
        for entity in &effects {
            commands.entity(entity).try_despawn();
        }
        previous.trace = Trace::default();
        previous.run = None;
        previous.positions.clear();
        return;
    };
    let visible = state.echoes.opened
        && state.echoes.page == Page::Shop
        && matches!(run.phase, api::Phase::Shop | api::Phase::Reward);
    if previous.run != Some(run.id) || !visible {
        for entity in &effects {
            commands.entity(entity).try_despawn();
        }
        previous.positions.clear();
    }
    previous.run = Some(run.id);
    if !visible {
        let _ = previous
            .trace
            .observe(run.id, run.revision, &run.economy, &run.shop.offers);
        return;
    }
    // Wait for the current revision's real layout. A network reply can arrive
    // before its replacement nodes exist; guessing positions causes jumps.
    if !places.iter().any(|(place, node, _)| {
        place.run == run.id
            && place.revision == run.revision
            && place.key == Anchor::Wallet
            && node.size().min_element() > 0.0
    }) {
        return;
    }
    let mut changes = previous
        .trace
        .observe(run.id, run.revision, &run.economy, &run.shop.offers);
    // Worker leases and record chunks advance several revisions during a duel.
    // Its settled income therefore cannot depend on adjacent shop revisions.
    if let Some(gold) = previous.income.observe(
        run.id,
        run.history.len(),
        run.reward.is_some(),
        baylee_client_core::echoes::domain::v2::DUEL_INCOME
            + run.history.last().map_or(0, |duel| duel.bonus_gold),
    ) {
        changes.get_or_insert_default().gold = i32::from(gold);
    }
    let layout_changed = places
        .iter()
        .any(|(_, node, transform)| node.is_changed() || transform.is_changed());
    if changes.is_none() && !layout_changed {
        return;
    }
    let mut current = BTreeMap::new();
    for (place, node, transform) in &places {
        if place.run == run.id && place.revision == run.revision && node.size().min_element() > 0.0
        {
            current.insert(
                place.key,
                transform.translation * node.inverse_scale_factor(),
            );
        }
    }
    let reduced = prefs
        .as_ref()
        .is_some_and(|prefs| prefs.all().reduce_motion);
    if let Some(change) = &changes
        && let Some(sounds) = sounds.as_mut()
    {
        emit_sound(change, run, sounds);
    }
    if let Some(change) = changes.filter(|_| !reduced) {
        // Cancel old flights when a newer confirmed command arrives. Cosmetic
        // entities are bounded even under rapid buying/reordered socket replies.
        for entity in &effects {
            commands.entity(entity).try_despawn();
        }
        render(
            &mut commands,
            fonts.as_deref(),
            &change,
            &previous.positions,
            &current,
        );
    }
    previous.positions = current;
}

fn point(positions: &BTreeMap<Anchor, Vec2>, anchor: Anchor) -> Option<Vec2> {
    positions
        .get(&anchor)
        .or_else(|| positions.get(&Anchor::Bag))
        .copied()
}

fn render(
    commands: &mut Commands,
    fonts: Option<&crate::hud::UiFonts>,
    change: &feedback::Change,
    before: &BTreeMap<Anchor, Vec2>,
    after: &BTreeMap<Anchor, Vec2>,
) {
    use crate::towerui::motion;
    for flight in &change.flights {
        if let (Some(from), Some(to)) = (point(before, flight.from), point(after, flight.to)) {
            let entity = motion::item(commands, flight.goods, from, to, flight.fusion);
            commands.entity(entity).insert(Transient);
        }
    }
    if let Some((item, goods)) = change.fused
        && let Some(position) = point(after, Anchor::Item(item))
    {
        let entity = motion::fusion(commands, goods, position);
        commands.entity(entity).insert(Transient);
    }
    if change.gold != 0
        && let Some(fonts) = fonts
    {
        let shelf = change.purchase.map_or(Anchor::Shelf, Anchor::Offer);
        let endpoints = if change.gold < 0 {
            (point(before, Anchor::Wallet), point(before, shelf))
        } else if let Some(sold) = change.sold {
            (
                point(before, Anchor::Item(sold)),
                point(after, Anchor::Wallet),
            )
        } else {
            (point(after, Anchor::Shelf), point(after, Anchor::Wallet))
        };
        if let (Some(from), Some(to)) = endpoints {
            for ordinal in 0..u8::try_from(change.gold.unsigned_abs().min(6)).unwrap_or(6) {
                let entity = motion::coin(commands, fonts, from, to, ordinal);
                commands.entity(entity).insert(Transient);
            }
        }
    }
}

fn emit_sound(
    change: &feedback::Change,
    run: &api::Run,
    sounds: &mut MessageWriter<crate::sound::tower::TowerSound>,
) {
    use crate::sound::tower::TowerSound;
    if change.fused.is_some() {
        sounds.write(TowerSound::Fusion);
    } else if change.purchase.is_some() {
        sounds.write(TowerSound::Buy);
    } else if change.sold.is_some() {
        sounds.write(TowerSound::Sell);
    } else if let Some(flight) = change.flights.first()
        && let Anchor::Item(id) = flight.to
    {
        let equipped = run
            .economy
            .inventory
            .relics
            .iter()
            .any(|r| r.id == id && matches!(r.slot, super::Slot::Active(_)));
        sounds.write(if equipped {
            TowerSound::Equip
        } else {
            TowerSound::Store
        });
    }
}
