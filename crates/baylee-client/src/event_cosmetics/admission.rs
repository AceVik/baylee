use super::{Paint, Worn};
use crate::{cardmat, lobby::advent::Book};
use baylee_account::advent::{AssetMeta, AssetScope, Finish, Id, Kind};
use baylee_client_core::images::FinishTreatment;
use bevy::prelude::*;

#[derive(Default, PartialEq, Eq)]
struct Slots {
    sleeve: Option<AssetMeta>,
    mat: Option<AssetMeta>,
}

/// Snapshot supplied only by the authenticated active-seat transport. An
/// empty resource selects resident artwork for spectators, replays and bots.
#[derive(Default, Resource, PartialEq, Eq)]
pub(crate) struct MatchLooks {
    table: Option<Id>,
    slots: [Slots; 8],
}

impl MatchLooks {
    pub(crate) fn participant(
        table: Id,
        slots: impl IntoIterator<Item = (u8, Kind, AssetMeta)>,
    ) -> Self {
        let mut result = Self {
            table: Some(table),
            ..default()
        };
        for (seat, kind, asset) in slots {
            if let Some(slot) = result.slots.get_mut(usize::from(seat)) {
                match kind {
                    Kind::Sleeve => slot.sleeve = Some(asset),
                    Kind::Playmat => slot.mat = Some(asset),
                }
            }
        }
        result
    }
    pub(crate) const fn table(&self) -> Option<Id> {
        self.table
    }
}

pub(super) const fn tier(finish: Finish) -> u8 {
    match finish {
        Finish::Standard => 0,
        Finish::Special => 1,
        Finish::Finale => 2,
    }
}

pub(super) fn refresh(
    looks: Res<MatchLooks>,
    book: Option<Res<Book>>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    duel: Option<Res<crate::Duel>>,
    mut previous: Local<(bool, super::framing::Roster)>,
    mut worn: ResMut<Worn>,
    materials: Option<ResMut<Assets<cardmat::CardMaterial>>>,
) {
    let Some(mut materials) = materials else {
        return;
    };
    let motion_changed = prefs.as_ref().is_some_and(Res::is_changed);
    let visible = looks.table.is_some()
        && book.is_some()
        && duel
            .as_ref()
            .is_some_and(|d| !d.watching && d.replay.is_none());
    let roster = super::framing::Roster::new(
        duel.as_ref()
            .filter(|_| visible)
            .and_then(|d| d.statics.as_ref())
            .into_iter()
            .flat_map(|s| &s.seats)
            .map(|s| (s.player, s.team)),
    );
    if visible == previous.0
        && previous.1 == roster
        && !looks.is_changed()
        && !book.as_ref().is_some_and(Res::is_changed)
        && !motion_changed
    {
        return;
    }
    *previous = (visible, roster);
    let motion = cardmat::motion_of(prefs.as_ref().is_some_and(|p| p.all().reduce_motion));
    for (seat, slot) in (0u8..).zip(&looks.slots) {
        let paint = |kind, meta: Option<&AssetMeta>| -> Option<Paint> {
            if !visible {
                return None;
            }
            let meta = meta?;
            let scope = AssetScope::Equipped {
                match_id: looks.table?,
                seat,
                kind,
            };
            let image = book.as_ref()?.scoped_image(&scope, meta)?.clone();
            Some(Paint {
                image,
                tier: tier(meta.tier),
            })
        };
        let index = usize::from(seat);
        let sleeve = paint(Kind::Sleeve, slot.sleeve.as_ref());
        if worn.0[index].sleeve.as_ref().map(|(p, _)| p) != sleeve.as_ref() {
            worn.0[index].sleeve = sleeve.map(|p| {
                let mut m = cardmat::material(
                    cardmat::CardLook::back(FinishTreatment::Plain),
                    Some(p.image.clone()),
                    Color::WHITE,
                    motion,
                );
                m.params.finish = match p.tier {
                    1 => 6,
                    2 => 7,
                    _ => 0,
                };
                (p, materials.add(m))
            });
        } else if motion_changed
            && let Some((_, handle)) = &worn.0[index].sleeve
            && materials
                .get(handle)
                .is_some_and(|m| m.params.motion.to_bits() != motion.to_bits())
            && let Some(mut m) = materials.get_mut(handle)
        {
            m.params.motion = motion;
        }
        let mat = slot.mat.as_ref().and_then(|meta| {
            Some(super::MatPaint {
                paint: paint(Kind::Playmat, Some(meta))?,
                pixels: Vec2::new(f32::from(meta.width), f32::from(meta.height)),
                framing: previous.1.framing(seat, meta.mat_framing.as_ref()),
            })
        });
        if worn.0[index].mat != mat {
            worn.0[index].mat = mat;
        }
    }
}

pub(super) fn leave(mut looks: ResMut<MatchLooks>) {
    if looks.table.is_some() {
        *looks = MatchLooks::default();
    }
}
