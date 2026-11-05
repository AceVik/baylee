//! Card names for the objects an action or a question refers to.

use std::collections::BTreeMap;

use baylee_core::ids::{ObjectId, PlayerId};
use baylee_view::PlayerView;

/// A seat as the game's log names it: `P1` is seat 0.
pub(super) fn seat(player: PlayerId) -> String {
    format!("P{}", u32::from(player.get()) + 1)
}

/// Every object some seat can see, by id, with the seat that controls it
/// when it is in a public zone: the seats' views together see both hands
/// and every public zone.
#[derive(Debug, Default)]
pub(super) struct Names(BTreeMap<ObjectId, (String, Option<PlayerId>)>);

impl Names {
    /// The names in `views`, one per seat.
    pub(super) fn of(views: &[PlayerView]) -> Self {
        let mut names = BTreeMap::new();
        for view in views {
            for card in &view.hand {
                names.insert(card.id, (card.name.clone(), None));
            }
            let public = view
                .battlefield
                .iter()
                .chain(&view.stack)
                .chain(view.graveyards.iter().flatten())
                .chain(view.exile.iter().flatten())
                .chain(view.command.iter().flatten())
                .chain(&view.looking_at)
                .chain(&view.library_tops);
            for object in public {
                names.insert(object.id, (object.name.clone(), Some(object.controller)));
            }
        }
        Self(names)
    }

    /// The name of `id`, or its number when no seat can see it.
    pub(super) fn name(&self, id: ObjectId) -> String {
        self.0
            .get(&id)
            .map_or_else(|| format!("#{id}"), |(name, _)| name.clone())
    }

    /// `text` (an action's or question's `Debug`) with every
    /// `ObjectId(slot#generation)` replaced by the object's name (and, in a
    /// public zone, `/P<n>` for its controller) and every `PlayerId(n)` by
    /// the seat's name.
    pub(super) fn annotate(&self, text: &str) -> String {
        let named = replace_ids(text, "ObjectId(", |body| {
            let (slot, generation) = body.split_once('#')?;
            let id = ObjectId::new(slot.parse().ok()?, generation.parse().ok()?);
            Some(match self.0.get(&id) {
                Some((name, Some(controller))) => format!("{name}/{}", seat(*controller)),
                Some((name, None)) => name.clone(),
                None => format!("#{id}"),
            })
        });
        replace_ids(&named, "PlayerId(", |body| {
            Some(seat(PlayerId::new(body.parse().ok()?)))
        })
    }
}

/// `text` with every `open…)` replaced by `name(…)`, or kept as it was
/// where `name` has none.
fn replace_ids(text: &str, open: &str, name: impl Fn(&str) -> Option<String>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(open) {
        out.push_str(&rest[..at]);
        let tail = &rest[at + open.len()..];
        let Some(end) = tail.find(')') else {
            out.push_str(&rest[at..]);
            return out;
        };
        match name(&tail[..end]) {
            Some(named) => out.push_str(&named),
            None => out.push_str(&rest[at..=at + open.len() + end]),
        }
        rest = &tail[end + 1..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_become_names_and_unknown_ones_stay_numbers() {
        let bolt = ObjectId::new(7, 0);
        let knight = ObjectId::new(8, 1);
        let names = Names(
            [
                (bolt, ("Lightning Bolt".to_owned(), None)),
                (knight, ("White Knight".to_owned(), Some(PlayerId::new(1)))),
            ]
            .into_iter()
            .collect(),
        );
        let text = format!(
            "CastSpell {{ card: {bolt:?} }} at {knight:?} or {:?} by PlayerId(0)",
            ObjectId::new(9, 2)
        );
        assert_eq!(
            names.annotate(&text),
            "CastSpell { card: Lightning Bolt } at White Knight/P2 or #9#2 by P1"
        );
    }

    #[test]
    fn an_unclosed_id_is_left_alone() {
        assert_eq!(Names::default().annotate("ObjectId(3#0"), "ObjectId(3#0");
    }
}
