//! A resolving color choice records a note without changing characteristics.
use super::{GameState, Pending, Resolution, source_version, subjects};
use baylee_core::{ids::DamageSourceRef, mana::ManaColor};

pub(super) fn choice(state: &GameState, res: &Resolution) -> Option<(DamageSourceRef, Pending)> {
    let source = DamageSourceRef {
        object: res.source,
        version: source_version(state, res)?,
    };
    subjects::on_battlefield(state, source).then(|| {
        (
            source,
            Pending::ChooseColor {
                player: res.controller,
                options: ManaColor::ALL[..5].to_vec(),
            },
        )
    })
}

pub(super) fn remember(state: &mut GameState, source: DamageSourceRef, color: ManaColor) {
    if !subjects::on_battlefield(state, source) {
        return;
    }
    state
        .object_mut(source.object)
        .expect("exact live source")
        .chosen_color = Some(color);
    state.invalidate_projections();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        engine::synthetic::{SyntheticLookup, preset},
        event::Cause,
        object::ObjectKind,
        zone::{ZoneLocation, ZonePosition},
    };
    use baylee_core::{
        color::{Color, ColorSet},
        ids::PlayerId,
    };

    #[test]
    fn a_color_note_preserves_characteristics_and_never_reaches_a_new_incarnation() {
        let mut state =
            GameState::from_preset(&preset(9321, &[]), &SyntheticLookup::new(vec![])).unwrap();
        let player = PlayerId::new(0);
        let name = state.names.intern("color note probe");
        let source = state.create_bare(
            player,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        state.object_mut(source).unwrap().base_mut().colors = ColorSet::of(Color::Green);
        let original = state.source_identity(source).unwrap();
        remember(&mut state, original, ManaColor::Red);
        assert_eq!(
            state.object(source).unwrap().chosen_color,
            Some(ManaColor::Red)
        );
        assert_eq!(
            state.object(source).unwrap().characteristics().colors,
            ColorSet::of(Color::Green)
        );
        remember(&mut state, original, ManaColor::Blue);
        assert_eq!(
            state.object(source).unwrap().chosen_color,
            Some(ManaColor::Blue)
        );
        state
            .move_object(
                source,
                ZoneLocation::Exile(player),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        state
            .move_object(
                source,
                ZoneLocation::Battlefield,
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        assert_eq!(state.object(source).unwrap().chosen_color, None);
        remember(&mut state, original, ManaColor::Black);
        assert_eq!(state.object(source).unwrap().chosen_color, None);
    }
}
