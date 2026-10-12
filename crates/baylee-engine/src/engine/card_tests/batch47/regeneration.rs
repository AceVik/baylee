//! The affected controller selects between standing and one-shot regeneration.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn batch47_clergy_controller_chooses_which_regeneration_replacement_applies() {
    for spell in [index::VINDICATE, index::LIGHTNING_BOLT] {
        for consume in [false, true] {
            let mut e = prepared(&[spell], &[index::CLERGY_OF_THE_HOLY_NIMBUS], &[]);
            let clergy = on_battlefield(&e, P0, index::CLERGY_OF_THE_HOLY_NIMBUS).unwrap();
            e.state.object_mut(clergy).unwrap().regeneration_shields = 1;
            let card = in_hand(&e, P0, spell).unwrap();
            e.refresh_offer();
            e.apply(P0, PlayerAction::CastSpell { card }).unwrap();
            targets(&mut e, vec![clergy], vec![]);
            pass_until(&mut e, |e| {
                matches!(
                    e.pending(),
                    Pending::YesNo {
                        prompt: crate::choice::YesNoPrompt::UseRegenerationShield { .. },
                        ..
                    }
                )
            });
            assert_eq!(e.pending().asked(), Some(P0));
            let before = e.snapshot_hash();
            assert!(e.apply(P1, PlayerAction::YesNo(consume)).is_err());
            assert_eq!(e.snapshot_hash(), before);
            e.apply(P0, PlayerAction::YesNo(consume)).unwrap();
            pass_until(&mut e, stack_is_empty);
            let object = e.state.object(clergy).unwrap();
            assert_eq!(object.zone, Zone::Battlefield);
            assert_eq!(object.regeneration_shields, u8::from(!consume));
            assert_eq!(object.damage, 0);
            assert!(is_tapped(&e, clergy));
            assert!(e.state.regeneration_choices.is_empty());
        }
    }
}
