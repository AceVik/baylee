use super::*;

/// The source of this file, down to where the tests begin: the crate's
/// modules in the order their declarations were written, then `lib.rs`.
fn declarations() -> &'static str {
    let source = concat!(
        include_str!("../turn.rs"),
        include_str!("../counters.rs"),
        include_str!("../status.rs"),
        include_str!("../game_static.rs"),
        include_str!("../objects.rs"),
        include_str!("../seats.rs"),
        include_str!("../combat.rs"),
        include_str!("../player_view.rs"),
        include_str!("../shown_hands.rs"),
        include_str!("../log.rs"),
        include_str!("../lib.rs"),
    );
    source
        .split_once("\n#[cfg(test)]")
        .map_or(source, |(head, _)| head)
}

/// Every line that decides the **shape** of what a client receives: the
/// `pub struct` and `pub enum` declarations, their fields and variants,
/// and the attributes on either. Doc comments and blank lines are not
/// shape, so writing down what a field means costs nothing.
fn wire_shape() -> Vec<String> {
    let mut shape: Vec<String> = Vec::new();
    let mut attributes: Vec<String> = Vec::new();
    let mut depth = 0usize;
    for line in declarations().lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        if depth > 0 {
            shape.push(line.to_string());
            depth += line.matches('{').count();
            depth = depth.saturating_sub(line.matches('}').count());
        } else if line.starts_with("#[") {
            attributes.push(line.to_string());
        } else if line.starts_with("pub struct ") || line.starts_with("pub enum ") {
            shape.append(&mut attributes);
            shape.push(line.to_string());
            depth = line.matches('{').count();
        } else {
            attributes.clear();
        }
    }
    shape
}

/// FNV-1a, spelled out, because this number is written down below.
/// `DefaultHasher` is documented as free to change between compiler
/// releases, and a recorded value that moved on a toolchain upgrade
/// would be a failure about nothing at all.
fn fingerprint(shape: &[String]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in shape.join("\n").bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Every `baylee_core` type this file names outside a comment, in its
/// `use` lines or by path: the types a client is sent inside the view's
/// own, whose shape [`wire_shape`] cannot read because it is written in
/// another crate.
fn core_types_named() -> std::collections::BTreeSet<String> {
    let is_type = |segment: &str| {
        segment.starts_with(|c: char| c.is_ascii_uppercase())
            && segment.contains(|c: char| c.is_ascii_lowercase())
    };
    let mut named = std::collections::BTreeSet::new();
    for line in declarations().lines().map(str::trim) {
        if line.starts_with("//") {
            continue;
        }
        if let Some(path) = line.strip_prefix("use baylee_core::") {
            let path = path.trim_end_matches(';');
            let list = path.split_once('{').map_or(path, |(_, group)| group);
            for item in list.trim_end_matches('}').split(',') {
                let item = item.trim().rsplit("::").next().unwrap_or("");
                if is_type(item) {
                    named.insert(item.to_string());
                }
            }
            continue;
        }
        for (at, prefix) in line.match_indices("baylee_core::") {
            let path: String = line[at + prefix.len()..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == ':')
                .collect();
            if let Some(ty) = path.split("::").find(|segment| is_type(segment)) {
                named.insert(ty.to_string());
            }
        }
    }
    named
}

/// What each of those types looks like on the wire, as a client reads
/// it: values serialized the way the view serializes them, one line per
/// type. A type's serde can change without a line of this file moving
/// (43 was one: [`ManaCost`] went from a slot list to its notation in
/// `baylee-core`), and its sample moves with it.
///
/// Named constants rather than raw numbers where a number means
/// something, so a renumbered subtype table moves the samples too — for
/// the three subtypes sampled. Every variant of an enum the view carries
/// is sampled, and the `match` beside each list stops the build on a new
/// one until it is added.
#[allow(clippy::too_many_lines)] // One sample table pins every external wire type.
fn core_samples() -> Vec<(&'static str, String)> {
    use baylee_core::color::Color;
    use baylee_core::generated::subtypes;
    use baylee_core::ids::SubtypeId;
    use baylee_core::mana::ManaColor;
    let json = |value: serde_json::Value| value.to_string();
    let defenders = [
        Defender::Player(PlayerId::new(1)),
        Defender::Planeswalker(ObjectId::new(12, 3)),
    ];
    for defender in defenders {
        match defender {
            Defender::Player(_) | Defender::Planeswalker(_) => {}
        }
    }
    for color in ManaColor::ALL {
        match color {
            ManaColor::White
            | ManaColor::Blue
            | ManaColor::Black
            | ManaColor::Red
            | ManaColor::Green
            | ManaColor::Colorless => {}
        }
    }
    let costs = mana_cost_samples();
    let mut subtypes_set = SubtypeSet::default();
    for id in [
        subtypes::creature::ALLY,
        subtypes::creature::ELF,
        subtypes::land::FOREST,
    ] {
        subtypes_set.insert(id);
    }
    let mut seats = SeatSet::new();
    seats.insert(PlayerId::new(0));
    seats.insert(PlayerId::new(2));
    vec![
        (
            "AbilityRef",
            json(serde_json::json!([
                AbilityRef::new(CardIndex::new(7), 1),
                AbilityRef::new(CardIndex::new(7), u32::MAX),
            ])),
        ),
        ("CardIndex", json(serde_json::json!(CardIndex::new(4096)))),
        (
            "TargetRef",
            json(serde_json::json!([
                TargetRef::Object(baylee_core::ids::DamageSourceRef {
                    object: ObjectId::new(7, 2),
                    version: 3
                }),
                TargetRef::Player(PlayerId::new(1)),
            ])),
        ),
        (
            "DamageSourceRef",
            json(serde_json::json!(baylee_core::ids::DamageSourceRef {
                object: ObjectId::new(12, 3),
                version: 70_001
            })),
        ),
        (
            "ColorSet",
            json(serde_json::json!([
                ColorSet::EMPTY,
                ColorSet::from_slice(&[Color::White, Color::Blue]),
                ColorSet::ALL,
            ])),
        ),
        ("Defender", json(serde_json::json!(defenders))),
        ("ManaColor", json(serde_json::json!(ManaColor::ALL))),
        ("ManaCost", json(serde_json::json!(costs))),
        ("ManaPayment", json(serde_json::json!(payment_samples()))),
        ("ManaSpending", json(serde_json::json!(spending_samples()))),
        ("ObjectId", json(serde_json::json!(ObjectId::new(12, 3)))),
        ("PlayerId", json(serde_json::json!(PlayerId::new(3)))),
        ("PrintRef", json(serde_json::json!(PrintRef::new(5)))),
        ("SeatSet", json(serde_json::json!(seats))),
        (
            "SubtypeId",
            json(serde_json::json!([
                subtypes::creature::ALLY,
                SubtypeId::new(0),
            ])),
        ),
        ("SubtypeSet", json(serde_json::json!(subtypes_set))),
        (
            "SupertypeSet",
            json(serde_json::json!([
                SupertypeSet::LEGENDARY,
                SupertypeSet::BASIC.union(SupertypeSet::SNOW),
            ])),
        ),
        (
            "TypeSet",
            json(serde_json::json!([
                TypeSet::CREATURE,
                TypeSet::ARTIFACT.union(TypeSet::LAND),
            ])),
        ),
    ]
}

fn mana_cost_samples() -> Vec<ManaCost> {
    let costs: Vec<ManaCost> = ["", "{2}{U}{U}", "{X}{R}", "{2}{W/U}{B/P}", "{16}{G}"]
        .iter()
        .map(|text| ManaCost::try_parse(text).expect("a cost the notation spells"))
        .collect();
    for cost in &costs {
        let back: ManaCost =
            serde_json::from_value(serde_json::to_value(cost).expect("serializes"))
                .expect("and reads back");
        assert_eq!(&back, cost, "a cost survives the wire");
    }
    costs
}

fn payment_samples() -> [ManaPayment; 2] {
    let payments = [
        ManaPayment::Fixed(ManaCost::parse("{2}{B}")),
        ManaPayment::AnyAmount {
            preventable_damage: 2,
        },
    ];
    for payment in payments {
        match payment {
            ManaPayment::Fixed(_) | ManaPayment::AnyAmount { .. } => {}
        }
    }
    payments
}

fn spending_samples() -> [baylee_core::mana::ManaSpending; 3] {
    use baylee_core::mana::{ManaColor, ManaSpending};
    let mut white_as_red = ManaSpending::EXACT;
    white_as_red.allow(ManaColor::White, ManaColor::Red);
    [ManaSpending::EXACT, ManaSpending::ANY_COLOR, white_as_red]
}

#[test]
fn an_old_stack_ability_payload_defaults_to_no_token_provenance() {
    let ability = StackItem::Ability {
        source: ObjectId::new(1, 0),
        ability: None,
        text: None,
        rules: None,
        token: Some(TokenAbility { token: 0, index: 0 }),
    };
    let mut wire = serde_json::to_value(ability).unwrap();
    wire["Ability"].as_object_mut().unwrap().remove("token");
    let restored: StackItem = serde_json::from_value(wire).unwrap();
    assert!(matches!(restored, StackItem::Ability { token: None, .. }));
}

/// **The shape on the wire and the number that names it move together.**
///
/// [`VIEW_VERSION`] is what lets a client refuse a host it cannot
/// render, and every reader of it in this workspace compares it against
/// itself — the gateway's two e2e tests, the client's handshake, a dozen
/// client fixtures. Not one of them can see the case the constant exists
/// for: a field added, renamed or retyped here while the number stays
/// where it was. Both ends then say 26 and one of them is wrong about
/// what 26 means, which is the failure this constant was introduced to
/// make impossible and the one failure it cannot catch by itself.
///
/// So the shape is recorded beside the version, and a change to either
/// stops here with both numbers in front of whoever made it. The
/// question it asks is the one [`VIEW_VERSION`]'s own doc comment asks:
/// was that breaking? If it was, bump the constant and say why in the
/// changelog above it. Either way, record the pair.
///
/// Every type in this file is `pub` — there are no others, so reaching
/// every declaration is reaching every field a client is sent.
///
/// The types a field here names from `baylee-core` are on the wire too,
/// and their shape is written there: 43 changed [`ManaCost`]'s serde and
/// not a line of this file. So each of them is sampled
/// ([`core_samples`]) into the same fingerprint, and every one this file
/// names has to have a sample ([`core_types_named`]).
///
/// This is a second guard and not a better one. What it cannot see is
/// written down one screen up: a renumbered
/// [`baylee_core::types::SubtypeSet`] leaves every struct in this file
/// exactly as it was, and two builds then agree on the shape and
/// disagree on what a number in it means. The samples see a renumbering
/// only where it moves one of the three subtypes they name.
#[test]
fn the_shape_on_the_wire_and_the_number_that_names_it_move_together() {
    // Damage decisions carry event identities, effect metadata, and allocations.
    // `PlayerView::sorceries_have_flash` is additive and defaulted: an
    // older client skips it and stays conservative, so 53 still names it.
    const RECORDED: (u32, u64) = (53, 6_263_270_473_425_543_018);

    let samples = core_samples();
    let sampled: std::collections::BTreeSet<String> =
        samples.iter().map(|(ty, _)| (*ty).to_string()).collect();
    assert_eq!(
        core_types_named(),
        sampled,
        "every baylee-core type this file names is sampled in \
         core_samples, and nothing else is: a type added to the view \
         from baylee-core needs its sample before its shape is guarded"
    );

    let mut shape = wire_shape();
    let declared = declarations().matches("\npub struct ").count()
        + declarations().matches("\npub enum ").count();

    assert!(
        shape.len() >= 150 && declared >= 15,
        "read {} lines and {declared} declarations out of this file — \
         the reader is broken, not the view. The equality below is \
         the door; this is only the floor",
        shape.len()
    );
    assert_eq!(
        shape
            .iter()
            .filter(|line| line.starts_with("pub struct ") || line.starts_with("pub enum "))
            .count(),
        declared,
        "every declaration in this file is one the reader reached"
    );
    shape.extend(samples.iter().map(|(ty, json)| format!("core {ty} {json}")));
    assert_eq!(
        (VIEW_VERSION, fingerprint(&shape)),
        RECORDED,
        "the view's shape and VIEW_VERSION no longer agree with what was \
         recorded here. If what changed is breaking for a client — a \
         field renamed, retyped or removed, a variant added to an enum a \
         client matches on — bump VIEW_VERSION and say why in the \
         changelog on it. Then record the pair above, whichever it was"
    );
}
