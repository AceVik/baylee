use super::*;
use baylee_core::ids::PlayerId;
use baylee_engine::choice::{DamageEffectKind, DamageEffectOption, DamagePartView};
use baylee_engine::event::DamageTarget;

#[test]
fn redirection_rows_describe_shares_without_promising_prevention() {
    let part = DamagePartView {
        id: 71,
        source: ObjectId::new(20, 0),
        recipient: DamageTarget::Player(PlayerId::new(0)),
        amount: 5,
        is_combat: true,
        preventable: false,
    };
    let prompt = Prompt::AllocatePrevention {
        effect: DamageEffectOption {
            id: 77,
            source: None,
            ability: None,
            controller: PlayerId::new(0),
            kind: DamageEffectKind::RedirectNext {
                remaining: 3,
                to: DamageTarget::Player(PlayerId::new(1)),
            },
            parts: vec![71],
        },
        damage: vec![part],
        total: 3,
        amounts: vec![2],
    };
    for (lang, share) in [
        (Lang::De, "leite 2 von höchstens 5 um"),
        (Lang::En, "redirect 2 of at most 5"),
    ] {
        let rows = options(&prompt, lang, None, "", FaceNames::default()).unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].label.contains(share), "{}", rows[0].label);
        assert!(
            rows[0]
                .label
                .contains(Phrase::DamageUnpreventable.text(lang))
        );
    }
}

#[test]
fn damage_rows_explain_the_offer_and_keep_bounded_pages() {
    let parts: Vec<_> = (0..9)
        .map(|id| DamagePartView {
            id: id + 10,
            source: ObjectId::new(id + 20, 0),
            recipient: DamageTarget::Player(PlayerId::new(0)),
            amount: u32::MAX,
            is_combat: id == 0,
            preventable: id != 1,
        })
        .collect();
    let effect = DamageEffectOption {
        id: 77,
        source: None,
        ability: None,
        controller: PlayerId::new(0),
        kind: DamageEffectKind::RemoveCounter {
            kind: baylee_cards_dsl::CounterKind::P1P1,
            remaining: 9,
        },
        parts: parts.iter().map(|part| part.id).collect(),
    };
    let prompt = Prompt::AllocatePrevention {
        effect,
        damage: parts,
        total: 9,
        amounts: vec![0; 9],
    };
    for lang in [Lang::En, Lang::De] {
        let mut rows = options(&prompt, lang, None, "", FaceNames::default()).unwrap();
        assert_eq!(rows.len(), 9);
        assert!(
            rows[1]
                .label
                .contains(Phrase::DamageUnpreventable.text(lang))
        );
        assert!(rows[0].label.contains("4294967295"));
        damage_page(&mut rows, 1, lang);
        assert_eq!(rows.len(), DAMAGE_PAGE_SIZE + 2);
        assert_eq!(rows[0].index, 4);
        assert_eq!(
            rows.last().unwrap().index,
            baylee_client_core::targeting::NEXT
        );
    }
}
