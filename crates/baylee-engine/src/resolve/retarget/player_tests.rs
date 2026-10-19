//! Retargeting replaces both representations of an announced player.
use super::*;
use crate::engine::synthetic::{SyntheticLookup, preset};
use crate::object::ObjectKind;
use crate::zone::ZoneLocation;

#[test]
fn mixed_target_replacement_updates_the_scalar_player_alias() {
    let you = PlayerId::new(0);
    let them = PlayerId::new(1);
    for spec in [
        TargetSpec::AnyTarget,
        TargetSpec::PlayerOrObject(&Filter::Any),
        TargetSpec::OpponentOrObject(&Filter::Any),
    ] {
        for to_object in [false, true] {
            let mut state =
                GameState::from_preset(&preset(594, &[]), &SyntheticLookup::new(vec![])).unwrap();
            let name = state.names.intern("retarget recipient");
            let recipient =
                state.create_bare(you, ObjectKind::Permanent, name, ZoneLocation::Battlefield);
            let name = state.names.intern("retargeted ability");
            let spell =
                state.create_bare(you, ObjectKind::AbilityOnStack, name, ZoneLocation::Stack);
            let obj = state.object_mut(spell).unwrap();
            obj.target_req = Some(TargetReq::one(spec));
            obj.target_players.insert(them);
            obj.chosen_player = Some(them);
            let next = if to_object {
                Aim::Object(state.source_identity(recipient).unwrap())
            } else {
                Aim::Player(you)
            };
            write(
                &mut state,
                &Retarget {
                    spell,
                    change_to: None,
                    was: vec![(0, Aim::Player(them))],
                    now: vec![Some(next)],
                    copy: false,
                    players_only: false,
                },
            );
            let obj = state.object(spell).unwrap();
            let players: Vec<_> = eval::targeted_players(obj, &spec).iter().collect();
            if to_object {
                assert!(players.is_empty());
                assert_eq!(obj.chosen_player, None);
                assert_eq!(obj.targets.as_slice(), &[recipient]);
            } else {
                assert_eq!(players, vec![you]);
                assert_eq!(obj.chosen_player, Some(you));
                assert!(obj.targets.is_empty());
            }
        }
    }
}
