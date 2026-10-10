//! Borrowed lookup tables for one board rebuild; never cached across snapshots.

use baylee_core::ids::{DamageSourceRef, ObjectId};
use baylee_view::{DamageSourceView, PlayerView, PublicObject};
use std::collections::HashMap;

/// Borrowed object and exact-target lookup for one immutable view.
/// The first visible occurrence wins, matching the view's linear queries.
pub struct Lookup<'a> {
    view: &'a PlayerView,
    objects: Option<HashMap<ObjectId, &'a PublicObject>>,
    targets: Option<HashMap<DamageSourceRef, &'a DamageSourceView>>,
}

impl<'a> Lookup<'a> {
    /// Index large views once; ordinary tables use their allocation-free queries.
    #[must_use]
    pub fn new(view: &'a PlayerView) -> Self {
        // Ordinary tables need only a handful of lookups. A deep stack pays
        // for these maps once instead of scanning each source and target.
        let indexed = view.stack.len() >= 256 || view.battlefield.len() >= 256;
        let objects = indexed.then(|| {
            let mut objects = HashMap::new();
            for object in view
                .battlefield
                .iter()
                .chain(&view.stack)
                .chain(view.graveyards.iter().flatten())
                .chain(view.exile.iter().flatten())
                .chain(view.command.iter().flatten())
                .chain(&view.looking_at)
                .chain(&view.library_tops)
            {
                // The visible projected object wins over a repeated reveal,
                // exactly as PlayerView::object's first match does.
                objects.entry(object.id).or_insert(object);
            }
            objects
        });
        let targets = indexed.then(|| {
            let mut targets = HashMap::with_capacity(view.target_objects.len());
            for target in &view.target_objects {
                targets.entry(target.source).or_insert(target);
            }
            targets
        });
        Self {
            view,
            objects,
            targets,
        }
    }

    /// The first public object with this handle in the view.
    #[must_use]
    pub fn object(&self, id: ObjectId) -> Option<&'a PublicObject> {
        self.objects
            .as_ref()
            .map_or_else(|| self.view.object(id), |objects| objects.get(&id).copied())
    }

    /// The public target snapshot for this exact incarnation.
    #[must_use]
    pub fn target(&self, source: DamageSourceRef) -> Option<&'a DamageSourceView> {
        self.targets.as_ref().map_or_else(
            || self.view.target_object(source),
            |targets| targets.get(&source).copied(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{ViewBuilder, target_snapshot, token};

    #[test]
    fn lookup_matches_first_match_and_exact_versions_in_all_zones() {
        for count in [0, 1, 255, 256, 1_000] {
            let mut view = ViewBuilder::new(2)
                .with_battlefield(0, (0..count).map(|n| token(n, 0, "Body", 1, 1)))
                .with_stack(
                    (0..count)
                        .map(|n| token(10_000 + n, 0, "Stack", 0, 0))
                        .collect(),
                )
                .build();
            view.graveyards[0].push(token(20_000, 0, "Grave", 1, 1));
            view.exile[0].push(token(20_001, 0, "Exile", 1, 1));
            view.command[0].push(token(20_002, 0, "Command", 1, 1));
            view.looking_at.push(token(20_003, 0, "Revealed", 1, 1));
            view.library_tops.push(token(20_004, 0, "Top", 1, 1));
            // A repeated reveal and a duplicate target must not replace the
            // first snapshot, even when a malformed view disagrees about it.
            view.looking_at.push(token(0, 0, "Duplicate", 9, 9));
            let mut old = target_snapshot(&view.looking_at[1], baylee_view::LogZone::Battlefield);
            view.target_objects.push(old.clone());
            old.source.version += 1;
            old.name = "New incarnation".into();
            view.target_objects.push(old);
            let lookup = Lookup::new(&view);
            for id in (0..count)
                .chain(10_000..10_000 + count)
                .chain(20_000..20_006)
            {
                let id = ObjectId::new(id, 0);
                assert_eq!(lookup.object(id), view.object(id));
                for version in [0, 1, 2, 3] {
                    let source = DamageSourceRef {
                        object: id,
                        version,
                    };
                    assert_eq!(lookup.target(source), view.target_object(source));
                }
            }
        }
    }
}
