use baylee_engine::object::{GameObject, ObjectKind, PrintedFace};
use baylee_view::RulesFace;

/// What a stack object is, for objects that are on the stack.
///
/// An ability on the stack is its own object with no card of its own, so
/// without this a client can only draw an anonymous entry — it knows a
/// trigger is resolving, but not whose, and not which of that permanent's
/// abilities it is. The engine already tracks exactly that in
/// `AbilityLoc`; this is where it reaches the client.
pub(crate) fn stack_item(obj: &GameObject) -> Option<baylee_view::StackItem> {
    use baylee_view::StackItem;
    match obj.kind {
        ObjectKind::Spell => Some(StackItem::Spell),
        ObjectKind::AbilityOnStack => obj.ability.map(|loc| {
            let provenance = obj
                .printed_ability_list(&crate::session::RegistryLookup)
                .origin(loc.index as usize);
            let printed = provenance
                .origin
                .and_then(baylee_engine::object::AbilityOrigin::printed);
            StackItem::Ability {
                token: provenance
                    .origin
                    .and_then(baylee_engine::object::AbilityOrigin::token)
                    .map(|id| baylee_view::TokenAbility {
                        token: id.get() - 1,
                        index: provenance.index,
                    }),
                source: loc.source,
                ability: provenance.ability_ref().or_else(|| {
                    (!baylee_core::ids::AbilityRef::new(
                        baylee_core::ids::CardIndex::new(0),
                        loc.index,
                    )
                    .is_listed_ability())
                    .then(|| {
                        loc.card
                            .map(|card| baylee_core::ids::AbilityRef::new(card, loc.index))
                    })
                    .flatten()
                }),
                text: printed.and_then(|printed| stack_text(printed, provenance.index)),
                rules: printed.map(rules_face),
            }
        }),
        _ => None,
    }
}

/// The view's spelling of the engine's [`PrintedFace`].
pub(crate) fn rules_face(face: PrintedFace) -> RulesFace {
    RulesFace {
        card: face.card(),
        face: face.face(),
    }
}

/// Where an ability's printed sentence is, for a client holding the card's
/// text in the player's own language.
///
/// Read against the card the ability is *printed on*, which the engine
/// carried beside the list the ability took with it (`GameObject::own_origin`)
/// — not the source's card, which for a copy is the wrong card, and not the
/// source's current face, which a transform may have turned since
/// (CR 113.7a). It used to be recovered from the list's address, and a copy
/// answered nothing because no face of the physical card matched; the
/// address was also never an identity, since two cards with byte-identical
/// lists share one.
///
/// Answers `None` for everything the generated table has no row for — a
/// reserved index (`AbilityRef::SPELL`, `SYNTHETIC`, …), a static ability,
/// a printed one no sentence fits — which is the whole point of it being
/// an `Option` on the wire. `docs/client.md` §"Which ability is on the
/// stack" is normative.
pub(crate) fn stack_text(printed: PrintedFace, index: u32) -> Option<baylee_view::StackText> {
    let line =
        baylee_cards::lines::ability_line(printed.card(), usize::from(printed.face()), index)?;
    Some(baylee_view::StackText {
        face: printed.face(),
        line: line.line,
        of: line.of,
    })
}
