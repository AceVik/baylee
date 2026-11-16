//! Distinct permanent medals, their exact targets and clearly labelled previews.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_client_core::{
    challenges::api::Tier,
    echoes::awards_api::{Badge, BadgeKind},
};

const KINDS: [BadgeKind; 3] = [
    BadgeKind::Victories,
    BadgeKind::Ascents,
    BadgeKind::Unbroken,
];
const TIERS: [Tier; 5] = [
    Tier::Bronze,
    Tier::Silver,
    Tier::Gold,
    Tier::Platinum,
    Tier::Obsidian,
];

pub(super) fn draw(commands: &mut Commands, root: Entity, kit: Kit, ui: &Ui) {
    let lang = lang(kit);
    let section = compact_panel(commands, root, kit);
    widgets::paragraph(
        commands,
        section,
        kit,
        Phrase::EchoesCollectionLead.text(lang),
        tokens::MUTED,
    );
    if ui.collection.pending {
        label(
            commands,
            section,
            kit,
            Phrase::ChallengeLoading.text(lang),
            tokens::ACCENT,
        );
    }
    if ui.collection.failed {
        label(
            commands,
            section,
            kit,
            Phrase::EchoesCollectionUnavailable.text(lang),
            tokens::DANGER,
        );
    }
    let gallery = row(commands, section, kit);
    commands
        .entity(gallery)
        .entry::<Node>()
        .and_modify(|mut n| n.align_items = AlignItems::Stretch);
    for kind in KINDS {
        let progress = ui.collection.progress.map(|p| {
            p.badges()
                .into_iter()
                .find(|b| b.kind == kind)
                .expect("three permanent tracks")
        });
        track(commands, gallery, kit, ui, kind, progress);
    }
    preview(commands, root, kit, ui);
}

fn track(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    ui: &Ui,
    kind: BadgeKind,
    badge: Option<Badge>,
) {
    let lang = lang(kit);
    let cell = cell(commands, root, kit, 240.0);
    medal(commands, cell, kit, kind, badge.and_then(|b| b.tier), 96.0);
    control(
        commands,
        cell,
        kit,
        ui,
        name(kind).text(lang),
        Action::Badge(kind),
        true,
    );
    centred(
        commands,
        cell,
        kit,
        badge.map_or("—", |b| tier(b.tier).text(lang)),
        tokens::GOLD,
    );
    let count = badge.map_or_else(
        || "—".into(),
        |b| {
            b.next.map_or_else(
                || Phrase::ChallengeBadgeMax.fill(lang, &[&b.count.to_string()]),
                |next| {
                    Phrase::ChallengeBadgeCompact
                        .fill(lang, &[&b.count.to_string(), &next.to_string()])
                },
            )
        },
    );
    centred(commands, cell, kit, &count, tokens::MUTED);
    if let Some(badge) = badge {
        meter(commands, cell, kit, badge);
    }
}

fn preview(commands: &mut Commands, root: Entity, kit: Kit, ui: &Ui) {
    let lang = lang(kit);
    let kind = ui.collection.selected;
    let surface = compact_panel(commands, root, kit);
    let title = row(commands, surface, kit);
    label(commands, title, kit, name(kind).text(lang), tokens::GOLD);
    label(
        commands,
        title,
        kit,
        Phrase::ChallengeTierPreview.text(lang),
        tokens::MUTED,
    );
    let goal = match kind {
        BadgeKind::Victories => Phrase::EchoesEarnVictories,
        BadgeKind::Ascents => Phrase::EchoesEarnAscents,
        BadgeKind::Unbroken => Phrase::EchoesEarnUnbroken,
    };
    widgets::paragraph(commands, surface, kit, goal.text(lang), tokens::INK);
    let gallery = row(commands, surface, kit);
    let count = ui.collection.progress.map(|p| match kind {
        BadgeKind::Victories => p.victories,
        BadgeKind::Ascents => p.clears,
        BadgeKind::Unbroken => p.flawless,
    });
    for (tier_value, target) in TIERS.into_iter().zip(kind.thresholds()) {
        let cell = cell(commands, gallery, kit, 144.0);
        medal(commands, cell, kit, kind, Some(tier_value), 88.0);
        centred(
            commands,
            cell,
            kit,
            tier(Some(tier_value)).text(lang),
            tokens::INK,
        );
        let earned = count.is_some_and(|count| count >= target);
        let caption = if earned {
            Phrase::ChallengeTierEarned
        } else {
            Phrase::ChallengeTierGoal
        };
        centred(
            commands,
            cell,
            kit,
            &caption.fill(lang, &[&target.to_string()]),
            if earned {
                tokens::ACCENT
            } else {
                tokens::MUTED
            },
        );
    }
}

fn cell(commands: &mut Commands, root: Entity, kit: Kit, basis: f32) -> Entity {
    let cell = commands
        .spawn((
            Node {
                flex_grow: 1.0,
                flex_basis: kit.m.px(basis),
                min_width: px(0),
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: kit.m.px(4.0),
                padding: UiRect::all(kit.m.px(6.0)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(cell);
    cell
}

fn medal(
    commands: &mut Commands,
    root: Entity,
    kit: Kit,
    kind: BadgeKind,
    tier: Option<Tier>,
    size: f32,
) {
    let medal = commands
        .spawn((
            crate::insignia::Insignia {
                kind: crate::insignia::Motif::Echoes(kind),
                tier,
            },
            Node {
                width: kit.m.px(size),
                height: kit.m.px(size),
                flex_shrink: 0.0,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(medal);
}

fn centred(commands: &mut Commands, root: Entity, kit: Kit, text: &str, color: Color) {
    let text = commands
        .spawn((
            Text::new(text),
            crate::hud::tf(kit.fonts, kit.m.small),
            TextColor(color),
            TextLayout::justify(Justify::Center),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(root).add_child(text);
}

fn meter(commands: &mut Commands, root: Entity, kit: Kit, badge: Badge) {
    // Convert only a bounded ratio; full u64 totals remain exact in the text.
    let share = badge.next.map_or(1.0, |next| {
        badge.count.min(next) as f64 / next.max(1) as f64
    });
    let track = commands
        .spawn((
            Node {
                width: percent(85),
                height: kit.m.px(3.0),
                ..default()
            },
            BackgroundColor(tokens::CONTROL),
            Pickable::IGNORE,
        ))
        .id();
    let fill = commands
        .spawn((
            Node {
                width: percent((share * 100.0) as f32),
                height: percent(100),
                ..default()
            },
            BackgroundColor(tokens::ACCENT),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(track).add_child(fill);
    commands.entity(root).add_child(track);
}

fn name(kind: BadgeKind) -> Phrase {
    match kind {
        BadgeKind::Victories => Phrase::EchoesBadgeVictories,
        BadgeKind::Ascents => Phrase::EchoesBadgeAscents,
        BadgeKind::Unbroken => Phrase::EchoesBadgeUnbroken,
    }
}

fn tier(tier: Option<Tier>) -> Phrase {
    match tier {
        None => Phrase::ChallengeNotEarned,
        Some(Tier::Bronze) => Phrase::ChallengeBronze,
        Some(Tier::Silver) => Phrase::ChallengeSilver,
        Some(Tier::Gold) => Phrase::ChallengeGold,
        Some(Tier::Platinum) => Phrase::ChallengePlatinum,
        Some(Tier::Obsidian) => Phrase::ChallengeObsidian,
    }
}
