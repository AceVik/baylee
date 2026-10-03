//! Labels for exact source incarnations, using only their entitled projection.
use crate::i18n::{Lang, Phrase};
use baylee_core::ids::DamageSourceRef;
use baylee_view::{GameStatic, PlayerView};
use std::fmt::Write as _;

/// Names an offered source without resolving its object to a newer incarnation.
#[must_use]
pub fn label(
    lang: Lang,
    source: DamageSourceRef,
    view: Option<&PlayerView>,
    statics: Option<&GameStatic>,
) -> String {
    label_named(lang, source, view, &|player| {
        crate::i18n::seat_name(lang, statics, player)
    })
}

/// Same projected identity, with a caller-provided roster naming function.
#[must_use]
pub fn label_named(
    lang: Lang,
    source: DamageSourceRef,
    view: Option<&PlayerView>,
    name: &impl Fn(baylee_core::ids::PlayerId) -> String,
) -> String {
    let Some(projected) = view.and_then(|v| v.damage_sources.iter().find(|s| s.source == source))
    else {
        return Phrase::SourceUnknown.text(lang).to_string();
    };
    describe(
        lang,
        projected,
        &projected.name,
        &name(projected.controller),
    )
}

/// Formats an entitled source snapshot; a renderer may localize its exact known name.
#[must_use]
pub fn describe(
    lang: Lang,
    projected: &baylee_view::DamageSourceView,
    source_name: &str,
    controller_name: &str,
) -> String {
    let source_name =
        if projected.card.is_none() && projected.rules.is_none() && projected.token.is_none() {
            match source_name {
                "Unknown source" => Phrase::SourceUnknown.text(lang),
                "Face-down" => Phrase::SourceFaceDown.text(lang),
                other => other,
            }
        } else {
            source_name
        };
    let state = if projected.is_current {
        Phrase::SourceCurrent
    } else {
        Phrase::SourceHistorical
    };
    let mut parts = vec![
        source_name.to_string(),
        controller_name.to_string(),
        state.text(lang).to_string(),
        zone_label(projected.zone).text(lang).to_string(),
    ];
    if !projected.is_current {
        parts.insert(
            2,
            Phrase::SourceVersion.fill(lang, &[&projected.source.version.to_string()]),
        );
    }
    if let (Some(power), Some(toughness)) = (projected.power, projected.toughness) {
        let _ = write!(parts[0], " ({power}/{toughness})");
    }
    if !projected.referenced_by.is_empty() {
        parts.push(
            Phrase::SourceReferences.fill(lang, &[&projected.referenced_by.len().to_string()]),
        );
    }
    parts.join(" — ")
}

fn zone_label(zone: baylee_view::LogZone) -> Phrase {
    use baylee_view::LogZone;
    match zone {
        LogZone::Battlefield => Phrase::BrowseBattlefield,
        LogZone::Graveyard => Phrase::BrowseGraveyard,
        LogZone::Exile => Phrase::BrowseExile,
        LogZone::Command => Phrase::BrowseCommand,
        LogZone::Hand => Phrase::SourceHand,
        LogZone::Library => Phrase::SourceLibrary,
        LogZone::Stack => Phrase::StackTitle,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{ViewBuilder, token};
    use baylee_core::ids::{ObjectId, PlayerId};

    fn projected(version: u32, current: bool, name: &str) -> baylee_view::DamageSourceView {
        baylee_view::DamageSourceView {
            source: DamageSourceRef {
                object: ObjectId::new(9, 0),
                version,
            },
            name: name.into(),
            card: None,
            rules: None,
            token: Some(1),
            controller: PlayerId::new(1),
            zone: baylee_view::LogZone::Battlefield,
            is_current: current,
            referenced_by: vec![ObjectId::new(10, 0)],
            colors: baylee_core::color::ColorSet::default(),
            types: baylee_core::types::TypeSet::default(),
            power: Some(2),
            toughness: Some(2),
            keywords: 0,
        }
    }

    #[test]
    fn exact_historical_projection_never_reads_a_newer_hidden_identity() {
        let mut view = ViewBuilder::new(2)
            .with_battlefield(1, vec![token(9, 1, "Different current identity", 8, 8)])
            .build();
        view.damage_sources = vec![
            projected(2, false, "Goblin"),
            projected(4, true, "Goblin"),
            projected(1, false, "Goblin"),
        ];
        let labels: Vec<_> = view
            .damage_sources
            .iter()
            .map(|s| label(Lang::En, s.source, Some(&view), None))
            .collect();
        assert!(labels[0].contains("earlier incarnation") && labels[0].contains("incarnation 2"));
        assert!(labels[1].contains("current"));
        assert_ne!(labels[0], labels[2]);
        assert!(
            labels
                .iter()
                .all(|s| !s.contains("Different current identity"))
        );
        let missing = DamageSourceRef {
            object: ObjectId::new(9, 0),
            version: 8,
        };
        assert_eq!(
            label(Lang::En, missing, Some(&view), None),
            "Unknown source"
        );
    }
    #[test]
    fn unknown_historical_sources_localize_without_identity_fallback() {
        let mut unknown = projected(1, false, "Unknown source");
        unknown.token = None;
        let mut view = ViewBuilder::new(2)
            .with_battlefield(1, vec![token(9, 1, "Secret card", 8, 8)])
            .build();
        view.damage_sources = vec![unknown.clone()];
        let text = label(Lang::De, unknown.source, Some(&view), None);
        assert!(text.starts_with("Unbekannte Quelle"));
        assert!(!text.contains("Secret card") && text.contains("früheres Objekt"));
        unknown.name = "Face-down".into();
        assert!(describe(Lang::De, &unknown, &unknown.name, "Ada").starts_with("Verdeckte Quelle"));
    }
}
