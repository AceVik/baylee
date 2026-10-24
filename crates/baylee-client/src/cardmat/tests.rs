use super::*;
use baylee_client_core::cardrail;

#[test]
fn game_materials_do_not_reuse_previous_games_print_references() {
    let mut cache = UiCardMaterials::default();
    let look = CardLook::art(
        ImageKey::new(
            baylee_core::ids::PrintRef::new(0),
            0,
            baylee_client_core::images::ArtSize::Small,
        ),
        FinishTreatment::Foil,
    );
    cache.made.insert(look, Handle::default());
    cache
        .previewed
        .insert(("url".into(), FinishTreatment::Foil), Handle::default());
    cache.clear_game();
    assert!(cache.made.is_empty());
    assert_eq!(
        cache.previewed.len(),
        1,
        "URL-based builder previews are independent of games"
    );
}

/// A permanent of a given type, with nothing else on it.
///
/// Spelled out here rather than borrowed from `client-core`'s builders,
/// which are `pub(crate)` there: what these tests need is three fields,
/// and a view is not what they are about.
fn permanent(types: baylee_core::types::TypeSet) -> baylee_view::PublicObject {
    use baylee_core::color::ColorSet;
    use baylee_core::ids::{ObjectId, PlayerId};
    use baylee_core::types::{SubtypeSet, SupertypeSet};
    baylee_view::PublicObject {
        battle_protector: None,
        prepared_spell: None,
        ability_sources: Vec::new(),
        word_changes: Vec::new(),
        mana_value: 0,
        id: ObjectId::new(1, 0),
        // A card, and it matters: `card: None` is a *token*, and every
        // test below that reads a keyword or an offer would be handed a
        // token mark along with it. The name is one no card is printed
        // with, so the registry answers nothing and this is an ordinary
        // permanent — which is what all of them meant by "a permanent".
        card: Some(baylee_view::CardIdentity {
            index: baylee_core::ids::CardIndex::new(0),
            print: baylee_core::ids::PrintRef::new(0),
            face: 0,
        }),
        rules: Some(baylee_view::RulesFace {
            card: baylee_core::ids::CardIndex::new(0),
            face: 0,
        }),
        name: "Test".to_string(),
        controller: PlayerId::new(0),
        owner: PlayerId::new(0),
        commander: false,
        status: baylee_view::ObjectStatus::NONE,
        types,
        supertypes: SupertypeSet::EMPTY,
        subtypes: SubtypeSet::EMPTY,
        chosen_subtype: None,
        chosen_name: None,
        chosen_opponent: None,
        unlocked_doors: None,
        suspended: false,
        token: None,
        colors: ColorSet::default(),
        keywords: 0,
        power: Some(2),
        toughness: Some(2),
        base_power: Some(2),
        base_toughness: Some(2),
        loyalty: None,
        damage: 0,
        counters: Vec::new(),
        attached_to: None,
        targets: Vec::new(),
        stack_item: None,
        summoning_sick: false,
        ability_mana_spending: baylee_core::mana::ManaSpending::EXACT,
        granted_mana: None,
        board_mana: None,
        flashback: None,
        grants: Vec::new(),
    }
}

/// The protection word speaks for three keywords; the engine numbers
/// over a hundred and generates that numbering. A card shelled for the
/// wrong keyword would be a rules lie a player would believe.
#[test]
fn the_glow_bits_are_the_keywords_they_claim_to_be() {
    use baylee_cards_dsl::KeywordSet;
    assert_eq!(
        glow_bits(KeywordSet::INDESTRUCTIBLE.bits()),
        glow::INDESTRUCTIBLE
    );
    assert_eq!(glow_bits(KeywordSet::HEXPROOF.bits()), glow::HEXPROOF);
    assert_eq!(glow_bits(KeywordSet::SHROUD.bits()), glow::SHROUD);
    // And nothing else reaches it: flying is a mark on the strip and
    // nothing more, and a keyword in this word would be claiming a
    // protection the card does not have.
    assert_eq!(glow_bits(KeywordSet::FLYING.bits()), 0);
    assert_eq!(glow_bits(0), 0);
}

/// Two protections on one card are one word with both bits.
#[test]
fn a_card_can_wear_more_than_one_glow() {
    use baylee_cards_dsl::KeywordSet;
    let both = KeywordSet::INDESTRUCTIBLE
        .union(KeywordSet::HEXPROOF)
        .bits();
    assert_eq!(
        glow_bits(both),
        glow::INDESTRUCTIBLE | glow::HEXPROOF,
        "an indestructible hexproof creature wears both"
    );
}

/// Shroud and hexproof on one card is shroud: nobody may target it, its
/// controller included, which is exactly what shroud says. Two sheaths
/// would claim two protections where there is one — and would cost a
/// second material for a border no player could tell from the first.
#[test]
fn shroud_swallows_hexproof() {
    use baylee_cards_dsl::KeywordSet;
    let both = KeywordSet::SHROUD.union(KeywordSet::HEXPROOF).bits();
    assert_eq!(glow_bits(both), glow::SHROUD);
    // And it takes nothing else with it: an indestructible shrouded
    // creature is still made of metal.
    let all = KeywordSet::SHROUD
        .union(KeywordSet::HEXPROOF)
        .union(KeywordSet::INDESTRUCTIBLE)
        .bits();
    assert_eq!(glow_bits(all), glow::SHROUD | glow::INDESTRUCTIBLE);
}

/// A defender is a mark on the strip and nothing more.
///
/// It used to be drawn twice — the mark, and a brick wall crossing the
/// card's face — and the wall went with #274, because the face is the
/// print and nothing of ours is painted on it. What is pinned is that a
/// defender says nothing round itself, since the mark is the strip's.
#[test]
fn a_defender_is_a_mark_on_the_strip_and_nothing_more() {
    use baylee_cards_dsl::KeywordSet;
    use baylee_client_core::board::KeywordBadge;
    use baylee_core::types::TypeSet;
    let slot = cardrail::slot_of(KeywordBadge::Defender).expect("defender rides the strip");

    let mut wall = permanent(TypeSet::CREATURE);
    wall.keywords = KeywordSet::DEFENDER.bits();
    assert_eq!(
        glow_of(Some(&wall), Offer::NONE),
        0,
        "a defender is its mark, and no second drawing"
    );
    assert_eq!(
        cardrail::mark_bits(wall.keywords),
        1 << slot,
        "and the mark is on the strip"
    );
}

/// Protection and an offer are separate bits, and a card that is both
/// wears both: they are drawn in different places on purpose.
#[test]
fn a_card_can_be_protected_and_useful_at_once() {
    use baylee_cards_dsl::KeywordSet;
    use baylee_core::types::TypeSet;
    let mut obj = permanent(TypeSet::CREATURE);
    obj.keywords = KeywordSet::HEXPROOF
        .union(KeywordSet::INDESTRUCTIBLE)
        .bits();
    assert_eq!(
        glow_of(Some(&obj), Offer::activatable(true)),
        glow::HEXPROOF | glow::INDESTRUCTIBLE | glow::ACTIVATABLE
    );
}

/// The material key has to separate what the shader draws differently and
/// nothing else, or a board of forty Islands stops being one material.
#[test]
fn a_look_is_shared_by_exactly_what_looks_the_same() {
    use baylee_client_core::images::ArtSize;
    use baylee_core::ids::PrintRef;

    let key = ImageKey::new(PrintRef(0), 0, ArtSize::Normal);
    let plain = CardLook::art(key, FinishTreatment::Plain);
    assert_eq!(plain, CardLook::art(key, FinishTreatment::Plain));
    assert_ne!(
        plain,
        CardLook::art(key, FinishTreatment::Foil),
        "a foil is not the same surface as a plain card"
    );
}

/// Colours that a player could not tell apart must not cost two
/// materials.
#[test]
fn two_colours_a_player_cannot_tell_apart_share_a_material() {
    let a = Color::srgb(0.5, 0.25, 0.125);
    let b = Color::srgb(0.5 + 1.0 / 2048.0, 0.25, 0.125);
    assert_eq!(
        CardLook::flat(a, FinishTreatment::Plain),
        CardLook::flat(b, FinishTreatment::Plain)
    );
    assert_ne!(
        CardLook::flat(a, FinishTreatment::Plain),
        CardLook::flat(Color::srgb(0.1, 0.2, 0.3), FinishTreatment::Plain)
    );
}
/// No fragment entry point takes a derivative after a return inside a
/// branch. A return under a per-pixel condition leaves the rest of the
/// function in non-uniform control flow, and Chrome's WGSL compiler
/// (Tint) refuses `fwidth`, `dpdx`, `dpdy` and implicit-level sampling
/// there; naga accepts it, so the native build and `check_wgsl` never
/// see it. Measured in the beta.6 QA: `felt.wesl` took `fwidth(table)`
/// after the tear's early returns, the felt pipeline was invalid in the
/// browser and the web client quit the moment a table opened.
///
/// A text scan, so it sees calls written in the entry point itself and
/// not those inside helpers it calls.
#[test]
fn no_fragment_takes_a_derivative_after_an_early_return() {
    const DERIVATIVES: [&str; 8] = [
        "fwidth(",
        "fwidthFine(",
        "fwidthCoarse(",
        "dpdx",
        "dpdy",
        "textureSample(",
        "textureSampleBias(",
        "textureSampleCompare(",
    ];
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/shaders");
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .expect("the shaders directory")
        .map(|e| e.expect("an entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "wesl"))
        .collect();
    entries.sort();
    let mut scanned = 0;
    let mut faults = Vec::new();
    for path in entries {
        let source = std::fs::read_to_string(&path).expect("a shader");
        // Comments say "return" and "fwidth" too; only code counts.
        let code: String = source
            .lines()
            .map(|line| line.split("//").next().unwrap_or(""))
            .collect::<Vec<_>>()
            .join("\n");
        let mut rest = code.as_str();
        while let Some(at) = rest.find("@fragment") {
            rest = &rest[at + "@fragment".len()..];
            let Some(open) = rest.find('{') else { break };
            let mut depth = 0usize;
            let mut end = rest.len();
            let mut early_return: Option<usize> = None;
            for (i, c) in rest[open..].char_indices() {
                match c {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            end = open + i;
                            break;
                        }
                    }
                    'r' if depth >= 2
                        && early_return.is_none()
                        && rest[open + i..].starts_with("return") =>
                    {
                        early_return = Some(open + i);
                    }
                    _ => {}
                }
            }
            scanned += 1;
            if let Some(from) = early_return {
                let after = &rest[from..end];
                for name in DERIVATIVES {
                    if after.contains(name) {
                        faults.push(format!(
                            "{}: `{name}` after a branch's return",
                            path.file_name().unwrap_or_default().to_string_lossy()
                        ));
                    }
                }
            }
            rest = &rest[end..];
        }
    }
    assert!(scanned >= 10, "only {scanned} fragment entry points found");
    assert!(faults.is_empty(), "{faults:#?}");
}

/// Parses and validates a shader the way `wgpu` will.
///
/// The shaders are WESL (bevy 0.20 composes them with the `wesl` crate),
/// and what they add to WGSL here is two things naga cannot see, both
/// stripped first: `import …;` lines, which bevy resolves against its own
/// modules and ours, and `constants::MATERIAL_BIND_GROUP`, which the
/// pipeline substitutes. What the imports bring in is stubbed by the
/// caller with the same shapes bevy declares, so a use that would not
/// type-check against the real ones does not type-check here either.
pub(crate) fn check_wgsl(source: &str, prelude: &str) {
    // An import statement runs to its `;`, which may be lines later.
    let mut in_import = false;
    let body: String = source
        .lines()
        .filter(|line| {
            in_import |= line.trim_start().starts_with("import ");
            let keep = !in_import;
            in_import &= !line.contains(';');
            keep
        })
        .collect::<Vec<_>>()
        .join("\n")
        .replace("constants::MATERIAL_BIND_GROUP", "3");
    let full = format!("{prelude}{body}");

    let module = naga::front::wgsl::parse_str(&full)
        .unwrap_or_else(|e| panic!("does not parse:\n{}", e.emit_to_string(&full)));
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .unwrap_or_else(|e| panic!("does not validate: {e:?}"));
}

/// The value of a `const` declared in a WGSL file.
///
/// Reading the shader's own text is the only way these constants can be
/// checked at all: one side is a Rust `const` the compiler knows about,
/// the other is a number in a `.wesl` file that nothing but the GPU ever
/// parses. A mismatch is silent, and looks like a rail drawn in the wrong
/// place on a machine nobody is testing on.
pub(crate) fn wgsl_const(source: &str, name: &str) -> f32 {
    let head = format!("const {name}:");
    let line = source
        .lines()
        .map(str::trim_start)
        .find(|line| line.starts_with(&head))
        .unwrap_or_else(|| panic!("no `{name}` in the shader"));
    let value = line
        .rsplit_once('=')
        .and_then(|(_, rhs)| rhs.trim().strip_suffix(';'))
        .unwrap_or_else(|| panic!("cannot read `{line}`"));
    // A ratio stays written as one — `63.0 / 88.0` says what it is and
    // `0.7159091` does not — so the one operator that appears in these
    // constants is evaluated here rather than banned from the shader.
    let number = |text: &str| -> f32 {
        let text = text.trim().trim_end_matches('u');
        // A mask is written as a mask — `0x7ff` says eleven bits and
        // `2047` says nothing — so the one other literal form these
        // constants use is read here too.
        if let Some(digits) = text.strip_prefix("0x") {
            return u32::from_str_radix(digits, 16)
                .unwrap_or_else(|_| panic!("`{text}` is not a number")) as f32;
        }
        text.parse()
            .unwrap_or_else(|_| panic!("`{text}` is not a number"))
    };
    match value.split_once('/') {
        Some((num, den)) => number(num) / number(den),
        None => number(value),
    }
}

/// The same reading for a colour, which [`wgsl_const`] cannot do: its
/// answer is one `f32` and every constant here is three.
///
/// Written as a second function rather than as a general one returning a
/// slice, because the two are asked different questions — a scalar that
/// is secretly a triple is a bug and a triple read as a scalar is a
/// panic, and keeping them apart means neither can quietly become the
/// other. It accepts only the fully-spelled form
/// `vec3<f32>(a, b, c)`; a shorthand `vec3(…)` or a splat panics here
/// instead of reading two of the three channels as zero.
pub(crate) fn wgsl_vec3(source: &str, name: &str) -> [f32; 3] {
    let head = format!("const {name}:");
    let line = source
        .lines()
        .map(str::trim_start)
        .find(|line| line.starts_with(&head))
        .unwrap_or_else(|| panic!("no `{name}` in the shader"));
    let body = line
        .rsplit_once('=')
        .and_then(|(_, rhs)| rhs.trim().strip_suffix(';'))
        .and_then(|rhs| rhs.trim().strip_prefix("vec3<f32>("))
        .and_then(|rhs| rhs.strip_suffix(')'))
        .unwrap_or_else(|| panic!("cannot read `{line}`"));
    let mut out = [0.0; 3];
    let mut seen = 0;
    for (slot, text) in body.split(',').enumerate() {
        assert!(slot < 3, "`{name}` has more than three channels");
        out[slot] = text
            .trim()
            .parse()
            .unwrap_or_else(|_| panic!("`{text}` in `{name}` is not a number"));
        seen += 1;
    }
    assert_eq!(seen, 3, "`{name}` has {seen} channels, not three");
    out
}

/// The strip is laid out twice — once in Rust so the pointer can
/// hit-test a mark and the table can place the quad, once in WGSL so the
/// GPU can draw one — and the two have to be the same strip. Nothing in
/// either compiler can notice that they are.
#[test]
fn the_strip_is_in_the_same_place_in_both_languages() {
    let src = include_str!("../shaders/card_common.wesl");
    for (name, ours) in [
        ("STRIP_PAD", cardrail::STRIP_PAD),
        ("MARK", cardrail::MARK),
        ("MARK_GAP", cardrail::MARK_GAP),
        ("SHADOW_MARGIN", cardrail::SHADOW_MARGIN),
        ("CARD_ASPECT", cardrail::CARD_ASPECT),
    ] {
        let theirs = wgsl_const(src, name);
        assert!(
            (ours - theirs).abs() < 1e-6,
            "{name}: {ours} here, {theirs} in the shader"
        );
    }
    assert_eq!(
        wgsl_const(src, "MARK_COUNT") as usize,
        cardrail::MARK_ORDER.len(),
        "the shader draws a different number of marks than the strip has"
    );
    assert!(
        (wgsl_const(src, "ROW_MAX") - cardrail::ROW_MAX).abs() < 1e-6,
        "the shader opens a row at a different length than the hit test"
    );
    for (name, ours) in [
        ("LABEL_CREST_SHIFT", cardrail::label::CREST_SHIFT),
        ("LABEL_CREST_BITS", cardrail::label::CREST_BITS),
    ] {
        assert_eq!(
            wgsl_const(src, name) as u32,
            ours,
            "{name}: the shader reads the label word differently"
        );
    }
    assert_eq!(
        wgsl_const(src, "LABEL_ITEMS") as usize,
        2 + cardrail::MARK_ORDER.len() + baylee_client_core::cardcrest::MAX_CRESTS,
        "the chip, the moon's empty place, every mark and the crests"
    );
}

/// Holding still changes the cards already made, and makes no new ones.
///
/// Two claims, and both are the kind that prose cannot hold. The first is
/// that the setting is not in [`CardLook`]: if it ever became part of the
/// key, the same look would mint a second material and the first would
/// live on with nothing to evict it — so the handle has to come back
/// *identical*. The second is that the change is written into the
/// materials rather than the cache being emptied: a cleared cache is only
/// refilled by whatever draws the card next, and a hand nobody is
/// touching is drawn once and left alone, so the preference would appear
/// to do nothing until the game moved.
///
/// `Assets` needs no render plugins, which is why this can be a unit test
/// at all.
#[test]
fn holding_still_rewrites_the_cards_already_made() {
    let mut cache = UiCardMaterials::default();
    let mut assets = Assets::<CardUiMaterial>::default();
    let look = CardLook::flat(Color::WHITE, FinishTreatment::Foil);

    // The clock is only ever one of two exact values, but `float_cmp` is
    // right in general and the file already has the idiom.
    let clock = |assets: &Assets<CardUiMaterial>, handle: &Handle<CardUiMaterial>| {
        assets.get(handle).expect("the material").params.motion
    };

    let handle = cache.get(look, None, Color::WHITE, &mut assets);
    let made = clock(&assets, &handle);
    assert!(
        (made - MOVING).abs() < f32::EPSILON,
        "a card starts out animating, not at {made}"
    );

    cache.set_still(true, &mut assets);
    let told = clock(&assets, &handle);
    assert!(
        (told - STILL).abs() < f32::EPSILON,
        "the material already made is still at {told}: it was discarded \
             rather than told"
    );

    let again = cache.get(look, None, Color::WHITE, &mut assets);
    assert_eq!(
        again, handle,
        "the same look gave a second material: the setting has leaked \
             into the cache key"
    );
    assert_eq!(assets.len(), 1, "one look, one material");
}

/// The field names of a WGSL struct, in declaration order.
fn wgsl_fields(source: &str, name: &str) -> Vec<String> {
    let head = format!("struct {name} {{");
    let body = source
        .split_once(&head)
        .unwrap_or_else(|| panic!("no `struct {name}` in the shader"))
        .1
        .split_once('}')
        .expect("an unterminated struct")
        .0;
    body.lines()
        .map(str::trim)
        .filter(|line| !line.starts_with("///") && !line.starts_with("//"))
        .filter_map(|line| Some(line.split_once(':')?.0.trim().to_string()))
        .collect()
}

/// `CardParams` is one struct written three times.
///
/// A uniform is bytes: nothing checks that the Rust field order and the
/// two WGSL declarations agree, and a mismatch has neither an error nor a
/// crash. Swapping the last two would feed `tint`'s red channel in as the
/// clock and the clock in as a colour — every card on the table drawn in
/// a wrong flat colour, animating at a speed that depends on how blue it
/// is.
///
/// What this pins is the pair of shaders against each other and against
/// the order written out below, which is the Rust struct's. Adding a
/// field to `CardParams` and forgetting either shader fails here; the one
/// hole left is changing all three of these and none of the Rust, which
/// no plausible edit does.
#[test]
fn card_params_is_the_same_struct_in_all_three_files() {
    // The order `#[derive(ShaderType)]` writes the bytes in.
    let ours = [
        "finish",
        "has_art",
        "strength",
        "motion",
        "sweep_at",
        "sweep_rate",
        "sweep_door",
        "face",
        "tint",
        "veil",
        "veil_at",
        "veil_rate",
    ];
    for (which, src) in [
        ("card.wesl", include_str!("../shaders/card.wesl")),
        ("card_ui.wesl", include_str!("../shaders/card_ui.wesl")),
    ] {
        let theirs = wgsl_fields(src, "CardParams");
        assert_eq!(theirs, ours, "{which} disagrees about `CardParams`");
    }
}

/// The sheen crosses a card once, the right way, and only when a card
/// has been given one.
///
/// Read out of the WGSL for the reason the sleep test is: the direction
/// and the one-shot-ness live in the shader, nothing in Rust can observe
/// them, and the fault they replace was invisible in exactly that way —
/// a six-second loop on every card in the hand, which a player reads as
/// the cards flickering at rest and which no test noticed for months.
#[test]
fn the_sheen_crosses_once_from_the_bottom_right() {
    let common = include_str!("../shaders/card_common.wesl");
    // `uv` is 0 at the top left and 1 at the bottom right on both axes,
    // so this diagonal is the travel and the band is across it.
    assert!(
        common.contains("let along = (uv.x + uv.y) * 0.5;"),
        "the band no longer runs along the card's diagonal"
    );
    // 1 → 0 as the phase advances: bottom right to top left, with a
    // margin at each end so the band starts and finishes off the card.
    assert!(
        common.contains("mix(1.0 + SWEEP_MARGIN, -SWEEP_MARGIN, phase)"),
        "the band no longer travels bottom right to top left"
    );
    // And it is a *shot*, not a cycle: outside its phase there is
    // nothing there at all.
    assert!(
        common.contains("if phase < 0.0 || phase > 1.0 {"),
        "the sweep is no longer bounded to one pass"
    );

    for (which, src) in [
        ("card.wesl", include_str!("../shaders/card.wesl")),
        ("card_ui.wesl", include_str!("../shaders/card_ui.wesl")),
    ] {
        assert!(
            src.contains(
                "let travel = select(0.0, sweep_amount(uv, phase), params.sweep_rate > 0.0);"
            ),
            "{which} draws a sheen on cards that were given none"
        );
        // And the sheen is all that is left of the metal. The coating
        // every card used to wear lifted the print's blacks, the artist's
        // line and the copyright line included, and the owner took it off
        // entirely (#274): light may pass over the print, but nothing may
        // stay on it.
        for gone in ["METAL_FLOOR", "METAL_GRAIN", "let brushed"] {
            assert!(!src.contains(gone), "{which} coats the print again: {gone}");
        }
    }
}

/// The five doors are the same five numbers on both sides of the wire.
///
/// Nothing in either compiler can notice that they are not: a
/// `sweep_door` of 4 is a valid `u32` whatever the shader thinks 4 means,
/// so a card being destroyed would simply be drawn as a bounce and the
/// build would stay green. This is the same pairing the rail gets, for
/// the same reason.
#[test]
fn the_doors_are_numbered_the_same_in_both_languages() {
    use baylee_client_core::zones::Passage;
    let src = include_str!("../shaders/card_common.wesl");
    for (name, ours) in [
        ("DOOR_NONE", door::NONE),
        ("DOOR_BOUNCE", door::BOUNCE),
        ("DOOR_EXILED", door::EXILED),
        ("DOOR_FLICKERED", door::FLICKERED),
        ("DOOR_DESTROYED", door::DESTROYED),
        ("DOOR_RETURNED", door::RETURNED),
    ] {
        let theirs = wgsl_const(src, name);
        assert!(
            (theirs - ours as f32).abs() < f32::EPSILON,
            "{name}: {ours} here, {theirs} in the shader"
        );
    }
    // And the Rust half maps every door to one of them. Written out
    // rather than swept, so a sixth `Passage` fails to compile here
    // instead of quietly mapping to `NONE` and drawing nothing.
    assert_eq!(door::code(None), door::NONE);
    assert_eq!(door::code(Some(Passage::Bounce)), door::BOUNCE);
    assert_eq!(door::code(Some(Passage::Exiled)), door::EXILED);
    assert_eq!(door::code(Some(Passage::Flickered)), door::FLICKERED);
    assert_eq!(door::code(Some(Passage::Destroyed)), door::DESTROYED);
    assert_eq!(door::code(Some(Passage::Returned)), door::RETURNED);
}

/// A door and the plain arrival are never drawn at once, and the pairs
/// are one figure reversed rather than two.
///
/// All of it read out of the WGSL, because none of it is observable from
/// Rust — and the failure it guards against is the quiet kind: a card
/// wearing both a white band and a violet ring reads as neither, and
/// looks in a screenshot exactly like a card wearing one of them.
#[test]
fn a_door_replaces_the_arrival_band_rather_than_joining_it() {
    for (which, src) in [
        ("card.wesl", include_str!("../shaders/card.wesl")),
        ("card_ui.wesl", include_str!("../shaders/card_ui.wesl")),
    ] {
        assert!(
            src.contains("let plain = select(0.0, travel, door == DOOR_NONE);"),
            "{which} lays the arrival band under a door"
        );
        assert!(
            src.contains(
                "let door = select(DOOR_NONE, params.sweep_door, params.sweep_rate > 0.0);"
            ),
            "{which} draws a door on a card that was given no sweep"
        );
    }
    let common = include_str!("../shaders/card_common.wesl");
    // The two exile doors are one ring, run in opposite directions — the
    // owner's "the same effect reversed" in one line each.
    assert!(
        common.contains("door_ring(uv, mix(DOOR_REACH, 0.0, phase))"),
        "exile no longer closes"
    );
    assert!(
        common.contains("door_ring(uv, mix(0.0, DOOR_REACH, phase))"),
        "a flicker no longer opens"
    );
    // And the two graveyard doors are one band, the same way.
    assert!(
        common.contains("door_band(uv.y, phase, true)"),
        "a destruction no longer falls down the card"
    );
    assert!(
        common.contains("door_band(uv.y, phase, false)"),
        "a resurrection no longer rises up it"
    );
    // A door outside its one pass draws nothing at all, which is what
    // keeps every card on the table paying one compare and no more.
    assert!(
        common.contains("if door == DOOR_NONE || phase < 0.0 || phase > 1.0 {"),
        "a door is no longer bounded to one pass"
    );
}

/// A card shader may only call what it has asked for by name.
///
/// `card_common.wesl` is imported with an explicit item list, so adding a
/// function to it and calling it from a card shader is *two* edits and
/// the second one is invisible: the file parses, every test that reads
/// the WGSL as text passes, and the composition fails at run time with
/// both card shaders dead — which draws the whole board wrong on the
/// first frame and nowhere else. `sweep_amount` shipped that way and was
/// caught by reading the import line, not by anything that ran.
#[test]
fn a_card_shader_imports_every_shared_helper_it_calls() {
    let common = include_str!("../shaders/card_common.wesl");
    let shared: Vec<&str> = common
        .lines()
        .filter_map(|line| line.strip_prefix("fn "))
        .filter_map(|rest| rest.split('(').next())
        .collect();
    assert!(
        shared.contains(&"sweep_amount"),
        "the shared file no longer defines the helpers this reads"
    );

    for (which, src) in [
        ("card.wesl", include_str!("../shaders/card.wesl")),
        ("card_ui.wesl", include_str!("../shaders/card_ui.wesl")),
    ] {
        let line = src
            .lines()
            .find(|line| line.starts_with("import baylee_client::shaders::card_common::"))
            .unwrap_or_else(|| panic!("{which} no longer imports the shared file"));
        let asked = line
            .split_once('{')
            .and_then(|(_, rest)| rest.split_once('}'))
            .expect("the import is no longer an item list")
            .0;
        let asked: Vec<&str> = asked.split(',').map(str::trim).collect();
        // The body is everything below the imports, so an import line
        // naming a function is not itself read as a call to it.
        let body = src.split_once("\nfn ").map_or(src, |(_, rest)| rest);
        for name in &shared {
            if body.contains(&format!("{name}(")) {
                assert!(
                    asked.contains(name),
                    "{which} calls {name} without importing it"
                );
            }
        }
    }
}

/// Every flag is a bit of its own, and every offer the same number on
/// both sides.
///
/// Two copies of the same table — one Rust, one WGSL — and nothing in
/// either compiler can notice when one of them moves. A wrong number here
/// has no error and no crash: the felt lights a card for an offer it does
/// not have, or two flags share a bit and one offer draws as another.
/// `floor_light` in `card_common.wesl` is the one reader (#298), and no
/// other shader may keep a table of its own that could drift from it.
#[test]
fn the_glow_flags_are_the_same_number_on_both_sides() {
    let common = include_str!("../shaders/card_common.wesl");
    let mut taken = 0u32;
    for (name, ours, drawn) in [
        ("GLOW_INDESTRUCTIBLE", glow::INDESTRUCTIBLE, false),
        ("GLOW_HEXPROOF", glow::HEXPROOF, false),
        ("GLOW_SHROUD", glow::SHROUD, false),
        ("GLOW_ACTIVATABLE", glow::ACTIVATABLE, true),
        ("GLOW_ARMED", glow::ARMED, true),
        ("GLOW_WILL_TAP", glow::WILL_TAP, true),
        ("GLOW_REACHABLE", glow::REACHABLE, true),
        ("GLOW_CHOOSABLE", glow::CHOOSABLE, true),
        ("GLOW_SELECTED", glow::SELECTED, true),
    ] {
        if drawn {
            let theirs = wgsl_const(common, name);
            assert!(
                (theirs - ours as f32).abs() < f32::EPSILON,
                "{name}: {ours} here, {theirs} in card_common.wesl"
            );
            assert_ne!(
                ours & glow::OFFERS,
                0,
                "{name} is an offer the felt is not lit for"
            );
        }
        assert_eq!(ours.count_ones(), 1, "{name} is not one bit");
        assert_eq!(ours & taken, 0, "{name} shares a bit with another flag");
        taken |= ours;
    }
    assert_eq!(
        glow::OFFERS.count_ones(),
        6,
        "the felt is lit for six offers, and the protection is not one"
    );
    for (which, src) in [
        ("card.wesl", include_str!("../shaders/card.wesl")),
        ("card_ui.wesl", include_str!("../shaders/card_ui.wesl")),
        ("floor.wesl", include_str!("../shaders/floor.wesl")),
    ] {
        assert!(
            !src.contains("const GLOW_"),
            "{which} keeps a glow table of its own beside the shared one"
        );
    }
}

/// Nothing is drawn on the print for good but its own finish (#274,
/// #298); what passes may (the owner, 13.10.2026).
///
/// The owner's rule, and Scryfall's: a card image is not covered, tinted,
/// dimmed or stamped, and the owner reads that as a lasting alteration
/// only. Each card shader samples the print edge to edge into `color`
/// and gives it its finish; after that the colour may be touched only by
/// the lamp and the arrival sweep (`METAL_TONE`), a door (`door_layer`),
/// a passing state (`veiled(`, [`veil`]: dimmed while a question leaves
/// the card out, phased out, asleep, each zero on every other card) and
/// the card's own corner — light that leaves nothing behind, a state
/// that ends, or ink outside the card. Everything lasting this client
/// says about a card stands on an object of its own: the strip, the
/// badge, the light on the felt, a keyword's border.
///
/// Read as text, because what it holds is a composition and not a
/// number: a rail or a night laid on the colour after the finish draws
/// perfectly well, compiles, and is exactly the fault. The live half —
/// the same card rendered with every state on and off, diffed over the
/// print — is in `docs/client.md` §"The print fills the card".
#[test]
fn nothing_but_the_finish_is_drawn_on_the_print() {
    for (which, src, angle) in [
        ("card.wesl", include_str!("../shaders/card.wesl"), "facing"),
        (
            "card_ui.wesl",
            include_str!("../shaders/card_ui.wesl"),
            "tilt",
        ),
    ] {
        let body = src
            .split_once("fn fragment(")
            .unwrap_or_else(|| panic!("{which} has no fragment"))
            .1;
        let lines: Vec<&str> = body.lines().map(str::trim).collect();
        let writes = |from: &[&'static str]| -> Vec<&'static str> {
            from.iter()
                .copied()
                .filter(|l| l.starts_with("color") || l.starts_with("var color"))
                .collect()
        };

        assert!(
            lines.contains(&"let sampled = textureSample(art, art_sampler, uv);"),
            "{which} samples its art somewhere other than edge to edge"
        );
        // What stands in for a print where there is none (#259): the flat
        // colour, or the text face. It is weighed by `1 - has_art`, so it
        // never lies on a print.
        let flats: Vec<&str> = lines
            .iter()
            .copied()
            .filter(|l| l.starts_with("let flat") || l.starts_with("flat ="))
            .collect();
        assert_eq!(
            flats,
            [
                "let flat = select(params.tint, vec4<f32>(text_face(uv, params.face), 1.0), (params.face & FACE_ON) != 0u);"
            ],
            "{which} stands something else in for a print"
        );

        // The print: sampled, finished, and nothing more.
        let finish =
            format!("color = print_finish(color, uv, {angle}, t, params.finish, params.strength);");
        let finished = lines
            .iter()
            .position(|l| *l == finish)
            .unwrap_or_else(|| panic!("{which} no longer finishes the print"));
        assert_eq!(
            writes(&lines[..finished]),
            ["var color = mix(flat, sampled, params.has_art);"],
            "{which} writes to the print before its finish"
        );
        let after = writes(&lines[finished + 1..]);
        assert!(
            after.len() >= 3,
            "only {} writes after the finish in {which} — the scan has gone blind",
            after.len()
        );
        // A statement split over lines is read by its lines: the opening
        // `color = vec4<f32>(` and the closing `color.a,` say nothing, and
        // the line between them has to name its light.
        for line in after {
            assert!(
                ["color = vec4<f32>(", "color.a,"].contains(&line)
                    || [
                        "METAL_TONE",
                        "door_layer(",
                        "veiled(",
                        "corner_sdf(uv)",
                        "EDGE_INK",
                    ]
                    .iter()
                    .any(|allowed| line.contains(allowed)),
                "{which} draws on the print after its finish: {line}"
            );
        }
        // The passing states are weighed by the card's own veil and
        // nothing else, which a card in none leaves at zero.
        let veiled = lines
            .iter()
            .filter(|l| l.contains("veiled("))
            .copied()
            .collect::<Vec<_>>();
        assert_eq!(
            veiled,
            [
                "color = veiled(color, uv, veil_amounts(params.veil, params.veil_at, params.veil_rate, globals.time), t);"
            ],
            "{which} lays a passing state on the print from something but its veil"
        );

        // And the frame's inputs are gone from the card with the frame.
        for field in ["params.glow", "params.plate", "params.chips", "print_cover"] {
            assert!(
                !src.contains(field),
                "{which} reads {field}: the card says that on an object of its own"
            );
        }
    }
}

/// An armed card is not also inviting a tap.
///
/// Both lights live in the same register on the border, and the whole
/// point of the pair is that one travels and one holds still. Drawing
/// both would put a chase and a steady ring on the same edge saying the
/// same thing twice, and a player would have nothing left to read the
/// difference from.
#[test]
fn arming_a_card_replaces_the_offer_it_accepted() {
    use baylee_core::types::TypeSet;
    let obj = permanent(TypeSet::CREATURE);
    let offer = Offer {
        activatable: true,
        reachable: false,
        armed: true,
        will_tap: false,
        choosable: false,
    };
    assert_eq!(glow_of(Some(&obj), offer), glow::ARMED);
    assert_eq!(
        glow_of(Some(&obj), Offer::activatable(true)),
        glow::ACTIVATABLE
    );
    // The price is a separate claim and rides alongside either of them:
    // the land being spent is not the card being cast.
    let paying = Offer {
        activatable: true,
        reachable: false,
        armed: false,
        will_tap: true,
        choosable: false,
    };
    assert_eq!(
        glow_of(Some(&obj), paying),
        glow::ACTIVATABLE | glow::WILL_TAP
    );
}

/// This client's offer is drawn only where nothing stronger is: the
/// engine's yes wins over it, and so does the deed it becomes once armed.
///
/// Three answers from one card, because each is a way to draw two lights
/// on one border — a chase in two hues at once, or a chase still running
/// round a card that has stopped inviting anything.
#[test]
fn a_reachable_card_is_lit_only_where_nothing_stronger_is() {
    use baylee_core::types::TypeSet;
    let obj = permanent(TypeSet::INSTANT);
    let reached = Offer::NONE.reaching(true);
    assert_eq!(glow_of(Some(&obj), reached), glow::REACHABLE);
    assert_eq!(
        glow_of(Some(&obj), Offer::activatable(true).reaching(true)),
        glow::ACTIVATABLE,
    );
    let armed = Offer {
        armed: true,
        ..reached
    };
    assert_eq!(glow_of(Some(&obj), armed), glow::ARMED);
}

/// A card standing for four permanents lights up when the deed touches
/// *any* of them.
///
/// The opposite of `CardGroup::activatable`, which is deliberately *all*,
/// and for a reason that does not transfer: that one invites a click, so
/// lighting a stack where only one member could act would invite a click
/// that gets refused. These two invite nothing — they announce what is
/// about to happen — and a stack of three Forests two of which are about
/// to tap is better drawn lit than dark.
#[test]
fn a_stack_is_lit_by_whichever_of_it_the_deed_touches() {
    use baylee_client_core::manaplan::{Plan, Step, Tap};
    let forests: Vec<ObjectId> = (1..=3).map(|i| ObjectId::new(i, 0)).collect();
    let elsewhere = ObjectId::new(9, 0);

    let run = crate::Armed {
        object: elsewhere,
        deed: crate::Deed::Run {
            plan: Plan {
                steps: vec![Step {
                    source: forests[1],
                    tap: Tap::Intrinsic,
                    color: None,
                }],
                ..default()
            },
            then: crate::RunEnd::Cast,
        },
    };
    let offer = Offer::on(crate::Proposing::Armed(&run), &forests, false);
    assert!(offer.will_tap, "one of the three is being spent");
    assert!(!offer.armed, "the spell is not one of the lands");

    // And a plan that touches none of them leaves the stack dark.
    let other = crate::Armed {
        object: elsewhere,
        deed: crate::Deed::Run {
            plan: Plan {
                steps: vec![Step {
                    source: elsewhere,
                    tap: Tap::Intrinsic,
                    color: None,
                }],
                ..default()
            },
            then: crate::RunEnd::Cast,
        },
    };
    assert_eq!(
        Offer::on(crate::Proposing::Armed(&other), &forests, true),
        Offer::activatable(true)
    );

    // A deed that is not a run spends nothing, whatever it is aimed at.
    let play = crate::Armed {
        object: forests[0],
        deed: crate::Deed::Play,
    };
    let offer = Offer::on(crate::Proposing::Armed(&play), &forests, false);
    assert!(offer.armed && !offer.will_tap);
}

/// The same question of an **owed** plan rather than an armed one, which
/// is the fourth corner of a square that had three.
///
/// `Armed` over a stack is the test above; `Owed` over a single land is
/// `owed_tests`; `Owed` over a card standing for several was held by
/// nothing. The two arms share one `spends` closure, so this is cheap —
/// and "cheap and untested" is exactly the shape #105 is about, so it is
/// written rather than argued.
#[test]
fn a_card_standing_for_several_lands_lights_when_an_owed_plan_taps_one() {
    use baylee_client_core::manaplan::{Plan, Step, Tap};
    let forests: Vec<ObjectId> = (1..=3).map(|i| ObjectId::new(i, 0)).collect();
    let plan = Plan {
        steps: vec![Step {
            source: forests[2],
            tap: Tap::Intrinsic,
            color: None,
        }],
        ..default()
    };
    let offer = Offer::on(
        crate::Proposing::Owed {
            plan: Some(&plan),
            staged: &[],
        },
        &forests,
        false,
    );
    assert!(offer.will_tap, "one of the three is named by the plan");
    assert!(!offer.armed, "an open window is not a commitment");

    let elsewhere = [ObjectId::new(9, 0)];
    assert_eq!(
        Offer::on(
            crate::Proposing::Owed {
                plan: Some(&plan),
                staged: &[]
            },
            &elsewhere,
            false
        ),
        Offer::activatable(false),
        "a card the plan does not name is drawn dark"
    );
}

/// Every mark the strip carries is the keyword it claims to be, the
/// three protections are marks too since #298 took the paper away, and
/// none of the combat marks reaches the protection word.
#[test]
fn the_strip_carries_the_keywords_it_says_it_does() {
    use baylee_cards_dsl::KeywordSet;
    let slot = |set: KeywordSet| cardrail::mark_bits(set.bits());
    assert_eq!(slot(KeywordSet::FLYING), 1 << 0);
    assert_eq!(slot(KeywordSet::DEATHTOUCH), 1 << 3);
    assert_eq!(slot(KeywordSet::DEFENDER), 1 << 10);
    // Prowess is bit 23 of the engine's word and slot 11 of the strip,
    // which is the whole reason the two numberings are pinned rather
    // than assumed to be the same list.
    assert_eq!(slot(KeywordSet::PROWESS), 1 << 11);
    // The paper's three were appended when the paper went (#298): the
    // index is the wire and the atlas cell, so they could not go in
    // front.
    assert_eq!(slot(KeywordSet::HEXPROOF), 1 << 12);
    assert_eq!(slot(KeywordSet::INDESTRUCTIBLE), 1 << 13);
    assert_eq!(slot(KeywordSet::SHROUD), 1 << 14);
    // And a creature wearing six combat keywords is one word with six
    // bits in it, and no protection.
    let six = KeywordSet::FLYING
        .union(KeywordSet::TRAMPLE)
        .union(KeywordSet::LIFELINK)
        .union(KeywordSet::VIGILANCE)
        .union(KeywordSet::HASTE)
        .union(KeywordSet::MENACE);
    assert_eq!(slot(six).count_ones(), 6, "six keywords, six marks");
    assert_eq!(glow_bits(six.bits()), 0);
}

/// A mark that takes the phase and never uses it has to say why.
///
/// Three did, when each mark was its own function: first strike, double
/// strike and defender all took `ph` and ignored it, and nothing in the
/// file said whether that was a decision or an omission — two were
/// omissions, the third is the design, and no reader could tell them
/// apart. The rail is the one place in this client where "it does not
/// animate" is both a legitimate answer and the signature of unfinished
/// work.
///
/// The twelve functions are gone and the question is not: `mark_pulse`
/// is one switch with an arm per mark, and an arm that returns a constant is
/// the same silence in one line instead of thirty. So a still arm
/// declares itself with `STILL` in the comment above it. This is the
/// build-time half of a claim that otherwise needs a camera: it cannot
/// say a motion is *visible*, which is what the measurements in that
/// file's header are for, but it can say that nobody added a mark and
/// quietly left the phase on the floor.
#[test]
fn the_rail_declares_every_mark_that_does_not_move() {
    let src = include_str!("../shaders/card_common.wesl");
    let open = src.find("fn mark_pulse(").expect("the pulse table");
    let body = &src[open..];
    let body = &body[..body.find("\n}").expect("a brace at column zero")];

    let mut seen = 0;
    let mut still = Vec::new();
    for (at, _) in body.match_indices("\n        case ") {
        let arm = &body[at + 1..];
        let slot: usize = arm["        case ".len()..]
            .split('u')
            .next()
            .expect("a slot number")
            .parse()
            .expect("a slot number");
        let end = arm
            .find("\n        }")
            .unwrap_or_else(|| arm.find('\n').expect("an arm ends on its own line"));
        let arm = &arm[..end];
        seen += 1;
        // `ph` as a token and not as a substring: `graph` is not a phase.
        let moves = arm.match_indices("ph").any(|(i, _)| {
            let before = arm[..i].chars().next_back();
            let after = arm[i + 2..].chars().next();
            let word = |c: char| c.is_alphanumeric() || c == '_';
            !before.is_some_and(word) && !after.is_some_and(word)
        });
        if moves {
            continue;
        }
        // The comment is the run of `//` lines above the arm. Measured
        // from the newline the arm starts on, not from the arm itself:
        // a slice that ends in `\n` splits to an empty last line and the
        // run stops before it has read anything.
        let note = body[..at]
            .rsplit('\n')
            .take_while(|line| line.trim_start().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            note.contains("STILL"),
            "slot {slot} ignores its phase and does not say why"
        );
        still.push(slot);
    }

    assert_eq!(
        seen,
        cardrail::MARK_ORDER.len(),
        "a mark on the strip, an arm"
    );
    assert_eq!(
        still,
        [10, 12, 13, 14],
        "which marks hold still is a decision, and this is the list of it"
    );
    assert_eq!(
        baylee_client_core::cardrail::MARK_ORDER[10],
        baylee_client_core::board::KeywordBadge::Defender,
        "slot 10 is the shield, and the stillness is drawn for a shield"
    );
}

/// Between choosing a mark and drawing its ink, nothing asks which mark
/// it is.
///
/// This is the half of the stillness claim that had no test, and #102 is
/// what it cost. `mark_pulse`'s slot 10 arm returns a flat `1.0` and
/// declares `STILL`, and
/// [`the_rail_declares_every_mark_that_does_not_move`] holds that —
/// honestly, because it is a test about a scale. But the ink under every
/// mark is `mix(INK, accent, 0.70) * (0.95 + 0.05 * sin(phase))` with no
/// `which` in it, so defender breathed with the rest of the row while two
/// comments and a test said it was the one thing on the rail that did
/// not move.
///
/// The number is why the answer was to correct the claim rather than add
/// the guard: that breath is 10.8 display levels peak to peak on slot
/// 10's warm stone, under a floor of 20. It is below what a player can
/// read, which makes it a false sentence and not a visible defect — and
/// makes a guard a change nobody could see.
///
/// So what is pinned here is the **absence**. `which` is read exactly
/// twice between the mark being chosen and the ink — the glyph and the
/// accent — and a third use is somebody making the drawing depend on the
/// mark's identity. That is a legitimate thing to want (#23 is open, and
/// #24's lane wave rides the same `phase`), which is precisely why it
/// should not be able to land quietly: this test going red *is* the
/// event, and whoever turns it green owes `mark_pulse`'s note the
/// sentence that is true afterwards.
#[test]
fn the_ink_below_a_mark_is_not_told_which_mark_it_is() {
    let src = include_str!("../shaders/card_common.wesl");
    let open = src.find("fn label_strip(").expect("the strip");
    let body = &src[open..];
    let body = &body[..body.find("\n}").expect("a brace at column zero")];

    // The window: after the line that names the mark the point is in, up
    // to and including the ink. Everything before it is *choosing* the
    // mark, and reading which one there is the point.
    let bail = body.find("let which = hit - 2u;").expect("the mark chosen");
    let after = body[bail..].find(';').expect("the line ends") + bail + 1;
    let ink = body.find("let ink =").expect("the ink");
    let ink = body[ink..].find(';').expect("the ink ends") + ink + 1;
    assert!(after < ink, "the mark is chosen before it is inked");

    // WGSL has no string literals, so a line is code up to its `//`.
    let code = body[after..ink]
        .lines()
        .map(|line| line.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");

    let uses = code
        .match_indices("which")
        .filter(|(i, _)| {
            let word = |c: char| c.is_alphanumeric() || c == '_';
            let before = code[..*i].chars().next_back();
            let after = code[i + "which".len()..].chars().next();
            !before.is_some_and(word) && !after.is_some_and(word)
        })
        .count();

    assert_eq!(
        uses, 2,
        "between the choice and the ink, `which` should be read exactly \
             twice — `mark_sdf(which, ..)` for the glyph and `mark_color(which)` \
             for the accent. {uses} means the ink now depends on which mark it \
             is drawing, which is #23's decision and #102's claim: say so in \
             `mark_pulse`'s note before making this number agree with you."
    );
}

/// The shader reads the atlas the baker writes.
///
/// Two numbers with no compiler between them, and each fails in its own
/// quiet way. A cell size out of step makes the half-texel inset the
/// wrong width, so a mark fetches a sliver of its neighbour along one
/// wall — which at ten pixels looks like a smudge and not like a bug.
/// A range out of step rescales every distance the rail has: the ink
/// edge moves, the halo changes size, and nothing anywhere errors.
#[test]
fn the_shader_and_the_baker_agree_about_the_atlas() {
    let src = include_str!("../shaders/card_common.wesl");
    assert!(
        (wgsl_const(src, "MARK_CELL") - crate::markatlas::CELL as f32).abs() < 1e-5,
        "one mark's square, in texels"
    );
    assert!(
        (wgsl_const(src, "MARK_RANGE") - crate::markatlas::RANGE).abs() < 1e-5,
        "how far either side of the outline the atlas encodes"
    );
    // And the atlas is exactly as many cells wide as the shader
    // indexes: it divides by `ATLAS_CELLS` to find a column, and the
    // corner's characters start at `TEXT_BASE` — so a cell added to
    // either half without the other number moving hands every glyph a
    // slice of its neighbour.
    let (wide, _) = crate::markatlas::atlas_size();
    assert_eq!(
        wide,
        wgsl_const(src, "ATLAS_CELLS") as u32 * crate::markatlas::CELL as u32
    );
    assert_eq!(
        wgsl_const(src, "TEXT_BASE") as usize + baylee_client_core::cardplate::TEXT_CHARS.len(),
        wgsl_const(src, "CREST_BASE") as usize,
        "the text half does not end where the identity column starts"
    );
    assert_eq!(
        wgsl_const(src, "CREST_BASE") as usize + baylee_client_core::cardcrest::GLYPH_COUNT,
        crate::markatlas::CELLS,
        "the identity column does not end where the row does"
    );
    assert_eq!(
        wgsl_const(src, "CREST_BASE") as usize,
        crate::markatlas::CREST_BASE,
        "the shader and the baker start the identity column in different cells"
    );
    assert!(
        (wgsl_const(src, "TEXT_RANGE") - baylee_client_core::cardplate::TEXT_RANGE).abs() < 1e-5,
        "how far either side of an outline a text cell encodes"
    );
}

/// The table shader, parsed and validated.
///
/// A WGSL error is otherwise found when a real pipeline is built — which
/// on the web is the one environment that cannot be debugged by looking
/// at a filesystem, and on native is a log line in a window that has
/// already drawn a black table. Naga is the same front end wgpu uses, so
/// what passes here compiles there.
#[test]
fn the_card_shader_compiles() {
    let prelude = "\
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};
struct View { world_position: vec3<f32> };
struct Globals { time: f32 };
@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(11) var<uniform> globals: Globals;
";
    check_wgsl(
        include_str!("../shaders/card.wesl"),
        &format!("{prelude}{}", include_str!("../shaders/card_common.wesl")),
    );
}

/// The plate is the same plate in Rust and in WGSL.
///
/// Thirty numbers with no compiler between them, and every one of them
/// fails silently: a slot boundary a bit out draws a 4/4 as a 0/16, a
/// glyph word off by a copy-paste draws every 6 as an 8, and a geometry
/// constant that drifts puts the plate off its quad. The
/// packing is checked from the other side by
/// `cardplate::tests::every_number_survives_the_packing`; this is the
/// half that checks the shader agrees about where the bits are.
#[test]
fn the_plate_is_the_same_plate_in_both_languages() {
    use baylee_client_core::cardplate as plate;
    let src = include_str!("../shaders/card_common.wesl");

    for (name, ours) in [
        ("PLATE_W", plate::PLATE_W),
        ("PLATE_H", plate::PLATE_H),
        ("PLATE_PAD", plate::PLATE_PAD),
        ("PLATE_CAP", plate::PLATE_CAP),
        ("CHIP_GAP", plate::CHIP_GAP),
        ("CHIP_W", plate::CHIP_W),
        ("BADGE_H", plate::BADGE_H),
        ("BADGE_CORNER", plate::BADGE_CORNER),
        ("BADGE_DROP_X", plate::BADGE_DROP[0]),
        ("BADGE_DROP_Y", plate::BADGE_DROP[1]),
        ("BADGE_BLUR", plate::BADGE_BLUR),
        ("BADGE_W", plate::BADGE_W),
    ] {
        let theirs = wgsl_const(src, name);
        assert!(
            (theirs - ours).abs() < 1e-5,
            "{name}: {ours} here, {theirs} in the shader"
        );
    }

    for (name, ours) in [
        ("PLATE_KIND_SHIFT", plate::KIND_SHIFT),
        ("PLATE_SLOT_BITS", plate::SLOT_BITS),
        ("PLATE_SLOT_MASK", plate::SLOT_MASK),
        ("PLATE_BIAS", plate::BIAS as u32),
        ("PLATE_NONE", plate::KIND_NONE),
        ("PLATE_FIGHT", plate::KIND_FIGHT),
        ("PLATE_LOYALTY", plate::KIND_LOYALTY),
        ("PLATE_LORE", plate::KIND_LORE),
        ("ROMAN_MAX", u32::from(plate::ROMAN_MAX)),
        ("TEXT_BASE", crate::markatlas::TEXT_BASE as u32),
        ("TEXT_COUNT", plate::TEXT_CHARS.len() as u32),
        ("ATLAS_CELLS", crate::markatlas::CELLS as u32),
        ("GLYPH_MINUS", plate::GLYPH_MINUS as u32),
        ("GLYPH_SLASH", plate::GLYPH_SLASH as u32),
        ("GLYPH_PLUS", plate::GLYPH_PLUS as u32),
        ("GLYPH_I", plate::GLYPH_I as u32),
        ("GLYPH_V", plate::GLYPH_V as u32),
        ("GLYPH_TIMES", plate::GLYPH_TIMES as u32),
        ("COUNT_MIN", plate::COUNT_MIN),
        ("COUNT_MAX", plate::COUNT_MAX),
        ("BADGE_ATTACHED", plate::BADGE_ATTACHED),
        ("SWING_SET", plate::SWING_SET),
        ("TONE_SHIFT", plate::TONE_SHIFT),
        ("TONE_PLAIN", plate::TONE_PLAIN),
        ("TONE_DEADLY", plate::TONE_DEADLY),
        ("TONE_TOXIC", plate::TONE_TOXIC),
        ("PLATE_NIGHT", plate::PLATE_NIGHT),
        ("PLATE_TURNED", plate::PLATE_TURNED),
    ] {
        // Half a unit, not an epsilon: these are whole numbers, so an
        // agreement is exactly zero apart and a disagreement is at least
        // one — and `f32::EPSILON` next to a seven-digit glyph word would
        // be asking for a precision no `f32` has up there.
        assert!(
            (wgsl_const(src, name) - ours as f32).abs() < 0.5,
            "{name} differs between the two files"
        );
    }

    // The corner's typesetting: what the shader has to know about the
    // face to lay a line of it out. An advance that drifted here would
    // not fail to draw — it would draw the same characters a fraction
    // out of step with one another, which is precisely the complaint
    // this whole change answers, so it is worth a mirror of its own.
    for (name, ours) in [
        ("TEXT_RANGE", plate::TEXT_RANGE),
        ("TEXT_CAP", plate::TEXT_CAP),
        ("TEXT_ADV_DIGIT", plate::TEXT_ADV[0]),
        ("TEXT_ADV_MINUS", plate::TEXT_ADV[plate::GLYPH_MINUS]),
        ("TEXT_ADV_SLASH", plate::TEXT_ADV[plate::GLYPH_SLASH]),
        ("TEXT_ADV_PLUS", plate::TEXT_ADV[plate::GLYPH_PLUS]),
        ("TEXT_ADV_I", plate::TEXT_ADV[plate::GLYPH_I]),
        ("TEXT_ADV_V", plate::TEXT_ADV[plate::GLYPH_V]),
        ("TEXT_ADV_TIMES", plate::TEXT_ADV[plate::GLYPH_TIMES]),
    ] {
        let theirs = wgsl_const(src, name);
        assert!(
            (theirs - ours).abs() < 1e-5,
            "{name}: {ours} here, {theirs} in the shader"
        );
    }
}

/// Parchment is one material, and the card and the interface draw it from
/// one number.
///
/// A saga's chapter page is parchment on a card; a prompt slip and the
/// ability sheet are parchment in the interface. They were two colours —
/// #E0D4B0 in the shader and #EDE3CC in the palette — which is a drift
/// nothing could see, because the two are never touching on screen. What
/// makes it worth a test is that they answer the same question: a player
/// who has learned what paper looks like in this game has learned one
/// thing, and two papers is a fact they have to hold twice.
///
/// Read out of the WGSL rather than restated here, the way the rail and
/// the plate are already held — a constant copied into a test agrees with
/// whatever it was copied from, including a mistake.
#[test]
fn the_parchment_is_the_same_paper_in_both_languages() {
    let src = include_str!("../shaders/card_common.wesl");
    let page = wgsl_rgb(src, "PARCHMENT");
    let sheet = crate::hud::palette::PARCHMENT.to_srgba();
    for (channel, (shader, ui)) in [
        (page[0], sheet.red),
        (page[1], sheet.green),
        (page[2], sheet.blue),
    ]
    .into_iter()
    .enumerate()
    {
        assert!(
            (shader - ui).abs() < 1e-6,
            "channel {channel}: the card draws parchment at {shader} and \
                 the interface paints it at {ui}"
        );
    }
    // And the generated sheet the interface stretches over that fill is
    // the same paper again in its middle, which is the seam a flat
    // border round a grained middle used to show.
    let grain = baylee_client_core::tabletop::parchment(64);
    let middle = grain.pixel(32, 32);
    for (channel, (fill, drawn)) in [
        (page[0], middle[0]),
        (page[1], middle[1]),
        (page[2], middle[2]),
    ]
    .into_iter()
    .enumerate()
    {
        assert!(
            (fill - drawn).abs() < 0.06,
            "channel {channel}: the fill is {fill} and the sheet over it \
                 is {drawn}"
        );
    }
}

/// One `vec3<f32>` literal out of the shader.
fn wgsl_rgb(source: &str, name: &str) -> [f32; 3] {
    let line = source
        .lines()
        .map(str::trim_start)
        .find(|line| line.starts_with(&format!("const {name}:")))
        .unwrap_or_else(|| panic!("the shader has no {name}"));
    let Some((inside, _)) = line
        .rsplit_once("vec3<f32>(")
        .and_then(|(_, tail)| tail.split_once(')'))
    else {
        panic!("{name} is not a vec3 literal: {line}")
    };
    let parts: Vec<f32> = inside
        .split(',')
        .map(|part| part.trim().parse().expect("a number"))
        .collect();
    [parts[0], parts[1], parts[2]]
}

/// Rec. 709 relative luminance — how bright a colour is, which is the
/// question "which of these two is the figure" actually asks.
fn luma(c: [f32; 3]) -> f32 {
    0.2126f32.mul_add(c[0], 0.7152f32.mul_add(c[1], 0.0722 * c[2]))
}

/// Hue in degrees, for the one comparison that is about colour rather
/// than brightness: a damaged creature must not be read as a
/// planeswalker.
fn hue(c: [f32; 3]) -> f32 {
    let high = c[0].max(c[1]).max(c[2]);
    let low = c[0].min(c[1]).min(c[2]);
    let span = high - low;
    assert!(span > 0.0, "a grey has no hue");
    let raw = if (high - c[0]).abs() < f32::EPSILON {
        (c[1] - c[2]) / span
    } else if (high - c[1]).abs() < f32::EPSILON {
        (c[2] - c[0]) / span + 2.0
    } else {
        (c[0] - c[1]) / span + 4.0
    };
    (raw * 60.0).rem_euclid(360.0)
}

/// How wide a card is drawn, in physical pixels, at the two distances
/// the plate has to work at.
///
/// Measured on this machine rather than derived: `/state` reports a
/// permanent on a duel's battlefield at about 47 logical pixels across,
/// and the hover preview asks for 308 before its own fit, both on a
/// display at scale 2. They are the whole reason the constants below
/// are what they are, so they are written down beside them.
const TABLE_CARD_PX: f32 = 94.0;
const PREVIEW_CARD_PX: f32 = 616.0;

/// Marked damage is drawn as the **figure** on the plate and the
/// numerals standing in it as its ground, and that is the whole of the
/// fix — not a louder colour.
///
/// It was a rising fill of `EMBER` at 58% *behind* near-white numerals,
/// which composites to a luminance of about 0.25 against the body's
/// 0.05 and the ink's 0.96: the brightest thing on the plate was the
/// numerals, so the numerals were what the eye read and the damage was
/// a slightly warmer dark behind them. The owner played whole games
/// without seeing it, which is the correct reading of that picture.
///
/// So the three claims here are about the ordering of brightnesses and
/// not about any one of them, and each is bounded on both sides: a band
/// far brighter than the body it covers, a digit inside it still well
/// clear of the band, and a hue nothing else in this corner owns.
#[test]
fn the_damage_band_is_the_figure_and_its_numerals_are_the_ground() {
    let src = include_str!("../shaders/card_common.wesl");
    let heat = wgsl_rgb(src, "HEAT");
    let plate = wgsl_rgb(src, "PLATE");
    let ink = wgsl_rgb(src, "INK");
    let gilt = wgsl_rgb(src, "GILT");

    // One number read twice, because a digit inside the band is drawn
    // in the plate's own colour: it is the band against the body it
    // covers, and it is the contrast that digit keeps standing in it.
    // Seven is the bound and these colours give nine — against the
    // nineteen a white digit keeps on the bare plate, and the four an
    // ink digit kept over the old 58% fill.
    let lit = luma(heat) / luma(plate);
    assert!(
        lit >= 7.0,
        "the band is only {lit} times the brightness of the plate, so \
             neither it nor the digits standing in it read"
    );

    // The other half of the inversion, and the one that says the
    // ordering rather than the ratio: the ink stays the brightest thing
    // the plate can draw, so a digit *outside* the band is still the
    // figure there. A band brighter than the ink would swap the two
    // back the other way round.
    assert!(
        luma(ink) > luma(heat),
        "the band is brighter than the ink it is read against"
    );

    // And laid on whole. The old fill was this colour at 58%, which is
    // the single edit that would put the damage back behind the
    // numerals while every constant above still passed.
    assert!(
        src.contains("out = mix(out, HEAT, band);"),
        "the band is no longer laid on at the band mask alone"
    );

    // Red of the gilt a planeswalker's plate is rimmed with, by enough
    // that the two are never one glance apart.
    let (hot, gold) = (hue(heat), hue(gilt));
    assert!(
        hot + 20.0 < gold,
        "the band sits at {hot}° against the rim's {gold}°"
    );
}

/// A point of damage is drawn at the table, and how *many* points is
/// answered where there is room to answer it.
///
/// Both halves are bounded on both sides, because both fail in two
/// directions: a floor too small says nothing and a floor too big makes
/// one damage on a twelve look like one damage on a two; a tick
/// threshold too high rules a plate eighteen pixels wide into invisible
/// rows and one too low never rules the preview at all.
#[test]
fn the_band_says_marked_at_the_table_and_how_much_in_the_preview() {
    use baylee_client_core::cardplate as plate;
    let src = include_str!("../shaders/card_common.wesl");
    let floor = wgsl_const(src, "DAMAGE_FLOOR");
    let tick_aa = wgsl_const(src, "TICK_AA");
    let tick_max = wgsl_const(src, "TICK_MAX");

    let pixels = floor * TABLE_CARD_PX;
    assert!(
        pixels >= 2.0,
        "one point of damage is {pixels} pixels tall at the table"
    );
    assert!(
        floor < plate::PLATE_H / 2.0,
        "the floor is {floor} of a {} plate, so a point of damage on a \
             big creature looks like a point on a small one",
        plate::PLATE_H
    );

    // `aa` is the pixel size in card widths, so the threshold is read
    // straight against the two distances.
    assert!(
        tick_aa < 1.0 / TABLE_CARD_PX,
        "the rules would be drawn on the table, {tick_aa} against {}",
        1.0 / TABLE_CARD_PX
    );
    assert!(
        tick_aa > 1.0 / PREVIEW_CARD_PX,
        "the preview is never ruled, {tick_aa} against {}",
        1.0 / PREVIEW_CARD_PX
    );

    // And the rows a ruled band can actually be counted in.
    let row = plate::PLATE_H / tick_max * PREVIEW_CARD_PX;
    assert!(
        row >= 3.0,
        "the tallest ruled creature has rows {row} pixels apart"
    );
}

/// The text face is the same face in both languages (#259).
///
/// The shader draws the bars and the table's `Text2d` lines stand on
/// them, placed by `textface`: a bar that drifted a hundredth in either
/// file puts a name half off its bar, on a machine where nothing fails.
/// The hues are read out of `face_hue`'s switch by their code, so a
/// shader that swapped two draws no blue card red.
#[test]
fn the_text_face_is_the_same_face_in_both_languages() {
    use baylee_client_core::textface::{self as face, Hue};
    let src = include_str!("../shaders/card_common.wesl");
    for (name, ours) in [
        ("TEXT_BORDER", face::BORDER),
        ("TEXT_SEAM", face::seam()),
        ("DEPTH_STEP", face::DEPTH_STEP),
        ("TEXT_PINLINE", face::PINLINE),
        ("TEXT_BOX_GAP", face::BOX_GAP),
        ("TEXT_FOOT", face::TEXT_FOOT),
        ("BAR_CORNER", face::BAR_CORNER),
        ("PAPER_MIX", face::PAPER_MIX),
        ("ART_TOP", face::ART_TOP),
        ("ART_FOOT", face::ART_FOOT),
        ("CLOTH", face::CLOTH),
        ("BEVEL", face::BEVEL),
        ("BAND_PREVIEW", face::BAND_PREVIEW),
        ("BAND_LONG", face::BAND_LONG),
        ("BAND_LIFT", face::BAND_LIFT),
        ("BAND_DEEP", face::BAND_DEEP),
        ("BAND_LUMA", face::BAND_LUMA),
        ("COST_LINE", face::COST_LINE),
        ("DISC_RADIUS", face::DISC_RADIUS),
        ("DISC_DROP", face::DISC_DROP),
        ("DISC_PAIR", face::DISC_PAIR),
    ] {
        let theirs = wgsl_const(src, name);
        assert!(
            (theirs - ours).abs() < 1e-6,
            "{name}: {ours} here, {theirs} in the shader"
        );
    }
    #[allow(clippy::cast_precision_loss)] // four small words
    for (name, ours) in [
        ("FACE_ON", face::FACE_ON),
        ("FACE_LAYOUT_SHIFT", face::FACE_LAYOUT_SHIFT),
        ("FACE_BARS_SHIFT", face::FACE_BARS_SHIFT),
        ("FACE_NAME_SHIFT", face::FACE_NAME_SHIFT),
        ("FACE_TYPE_SHIFT", face::FACE_TYPE_SHIFT),
        ("LAYOUT_TABLE", face::Layout::Table as u32),
        ("LAYOUT_PREVIEW", face::Layout::Preview as u32),
        ("LAYOUT_LONG", face::Layout::Long as u32),
        ("LAYOUT_SMALL", face::Layout::Small as u32),
    ] {
        let theirs = wgsl_const(src, name);
        assert!(
            (theirs - ours as f32).abs() < 1e-6,
            "{name}: {ours} here, {theirs} in the shader"
        );
    }
    let hues = [
        (Hue::White, "FACE_WHITE"),
        (Hue::Blue, "FACE_BLUE"),
        (Hue::Black, "FACE_BLACK"),
        (Hue::Red, "FACE_RED"),
        (Hue::Green, "FACE_GREEN"),
        (Hue::Gold, "FACE_GOLD"),
        (Hue::Grey, "FACE_GREY"),
    ];
    let tones = hues.iter().map(|(hue, name)| (*name, hue.tone())).chain([
        ("PAPER_WHITE", face::PAPER_WHITE),
        ("BORDER_INK", face::BORDER_INK),
    ]);
    for (name, ours) in tones {
        let theirs = wgsl_vec3(src, name);
        for c in 0..3 {
            assert!(
                (theirs[c] - ours[c]).abs() < 1e-5,
                "{name}: {ours:?} here, {theirs:?} in the shader"
            );
        }
    }
    // Each bar is drawn as deep as its own byte says, the byte the text
    // was placed by, and each part where `Regions::laid` puts it for the
    // layout the word names.
    for line in [
        "let name_end = top + f32((word >> FACE_NAME_SHIFT) & 0xffu) * DEPTH_STEP;",
        "let type_depth = f32((word >> FACE_TYPE_SHIFT) & 0xffu) * DEPTH_STEP;",
        "let face_layout = (word >> FACE_LAYOUT_SHIFT) & 0x7u;",
        "var text_top = TEXT_SEAM + type_depth + TEXT_BOX_GAP;",
        "type_top = name_end + TEXT_PINLINE;",
        "let band_top = type_top + type_depth + TEXT_PINLINE;",
        "text_top = TEXT_FOOT - type_depth;",
        "band = vec4<f32>(x0, band_top, x1, text_top - TEXT_BOX_GAP);",
        "let depth = select(BAND_PREVIEW, BAND_LONG, face_layout == LAYOUT_LONG);",
        "text_top = band.w + TEXT_BOX_GAP;",
        "let y = band.y + COST_LINE + DISC_DROP + DISC_RADIUS;",
        "return clamp(BAND_LUMA / max(luma, 0.0001), BAND_DEEP, 1.0);",
        "let luma = dot(hue, vec3<f32>(0.2126, 0.7152, 0.0722));",
    ] {
        assert!(src.contains(line), "`text_face` no longer says `{line}`");
    }
    // Grey is the switch's default, so a code the table does not know
    // is drawn as no colour rather than as white.
    for (hue, name) in hues {
        let arm = if hue == Hue::Grey {
            format!("default: {{ return {name}; }}")
        } else {
            format!("case {}u: {{ return {name}; }}", hue as u32)
        };
        assert!(src.contains(&arm), "`face_hue` has no `{arm}`");
    }
}

/// The face word a look carries reaches the shader's uniform, and only a
/// look that asked for the face carries one: a print and a back leave
/// the window to their picture (#259).
#[test]
fn a_face_look_carries_its_word_to_the_uniform() {
    use baylee_client_core::images::ArtSize;
    use baylee_client_core::textface::{FACE_NAME_SHIFT, FACE_ON};
    use baylee_core::ids::PrintRef;

    let tint = Color::srgb(0.2, 0.3, 0.4);
    let word = FACE_ON | 5 << 4 | 133 << FACE_NAME_SHIFT;
    let face = CardLook::flat(tint, FinishTreatment::Plain).with_face(word);
    assert_eq!(material(face, None, tint, 1.0).params.face, word);
    assert_ne!(face, CardLook::flat(tint, FinishTreatment::Plain));

    let key = ImageKey::new(PrintRef(0), 0, ArtSize::Normal);
    for look in [
        CardLook::flat(tint, FinishTreatment::Plain),
        CardLook::art(key, FinishTreatment::Plain),
        CardLook::back(FinishTreatment::Plain),
    ] {
        assert_eq!(material(look, None, tint, 1.0).params.face, 0, "{look:?}");
    }
}

/// The crests' papers and ink are the same in both languages, and the
/// crests are where the atlas put them.
///
/// The papers are keyed by the glyph index rather than written out in
/// order, so a shader that swapped two of them fails here rather than
/// shipping a copy on verdigris.
#[test]
fn the_identity_papers_are_the_same_in_both_languages() {
    use baylee_client_core::cardcrest as crest;

    let src = include_str!("../shaders/card_common.wesl");
    for (name, ours) in [
        ("CREST_BASE", crate::markatlas::CREST_BASE),
        ("CREST_TOKEN", crest::GLYPH_TOKEN),
        ("CREST_COPY", crest::GLYPH_COPY),
        ("CREST_COMMANDER", crest::GLYPH_COMMANDER),
    ] {
        assert!(
            (wgsl_const(src, name) - ours as f32).abs() < 0.5,
            "{name} differs between the two files"
        );
    }
    for (name, ours) in [
        ("PAPER_TOKEN", crest::PAPER[crest::GLYPH_TOKEN]),
        ("PAPER_COPY", crest::PAPER[crest::GLYPH_COPY]),
        ("PAPER_COMMANDER", crest::PAPER[crest::GLYPH_COMMANDER]),
        ("CREST_INK", crest::CREST_INK),
    ] {
        let theirs = wgsl_vec3(src, name);
        for c in 0..3 {
            assert!(
                (theirs[c] - ours[c]).abs() < 1e-5,
                "{name}: {ours:?} here, {theirs:?} in the shader"
            );
        }
    }
}

/// The rail's own file, which is plain WGSL and needs nothing stubbed.
///
/// It is checked alone as well as inside the two that import it, because
/// alone is how bevy compiles it: a module that failed only in isolation
/// would take both card shaders down with it and blame whichever pipeline
/// happened to be built first.
#[test]
fn the_marks_module_compiles() {
    check_wgsl(include_str!("../shaders/card_common.wesl"), "");
}
/// The UI twin, held to the same standard. Its bind group is group 1 —
/// `bevy_ui_render` puts the view layout first — and it reads `globals`
/// out of group 0, so the stubs are shaped for that.
#[test]
fn the_ui_card_shader_compiles() {
    let prelude = "\
struct UiVertexOutput {
    @location(0) uv: vec2<f32>,
    @location(1) border_widths: vec4<f32>,
    @location(2) border_radius_x: vec4<f32>,
    @location(3) border_radius_y: vec4<f32>,
    @location(4) @interpolate(flat) size: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};
struct Globals { time: f32 };
";
    check_wgsl(
        include_str!("../shaders/card_ui.wesl"),
        &format!("{prelude}{}", include_str!("../shaders/card_common.wesl")),
    );
}

/// This cache hands out one material per look and never looks at the
/// handle again, so a card back drawn before the picture arrived would
/// stay a flat colour for the rest of the session — the hand can be
/// rebuilt a hundred times and get the same material every time. The
/// picture is therefore written into what is already hanging there.
#[test]
fn a_back_made_before_the_picture_arrived_is_dressed_where_it_hangs() {
    let mut images = Assets::<Image>::default();
    let mut assets = Assets::<CardUiMaterial>::default();
    let mut cache = UiCardMaterials::default();

    // Two shapes, both legitimate: the hand builds a back around the
    // stand-in texture, and a caller with nothing to hand it builds one
    // around no picture at all. The second is the one that needs the
    // flag as well as the handle.
    let flat = images.add(Image::default());
    let back = cache.get(
        CardLook::back(FinishTreatment::Plain),
        Some(flat),
        Color::BLACK,
        &mut assets,
    );
    let bare = cache.get(
        CardLook::back(FinishTreatment::Plain),
        None,
        Color::BLACK,
        &mut assets,
    );
    // A card drawing its own text is not a back, and must not be given
    // the picture: both have no `ImageKey`, and the tint is what tells
    // them apart.
    let face = cache.get(
        CardLook::flat(Color::srgb(0.2, 0.3, 0.4), FinishTreatment::Plain),
        None,
        Color::srgb(0.2, 0.3, 0.4),
        &mut assets,
    );

    let printed = images.add(Image::default());
    assert!(!cache.backs_are_dressed());
    cache.dress_the_backs(&printed, &mut assets);
    assert!(cache.backs_are_dressed());

    let dressed = assets.get(&back).expect("the back material");
    assert_eq!(dressed.art.as_ref(), Some(&printed), "still the stand-in");
    let bare = assets.get(&bare).expect("the bare back material");
    assert_eq!(bare.art.as_ref(), Some(&printed));
    assert!(
        (bare.params.has_art - 1.0).abs() < f32::EPSILON,
        "given the picture and not the flag, it draws the tint as before"
    );
    assert_ne!(
        assets.get(&face).expect("the face material").art.as_ref(),
        Some(&printed),
        "a constructed face was dressed as a card back"
    );
}

/// The directory the client's shaders live in.
fn shader_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/shaders")
}

/// The files in the shader directory with the given extension, sorted.
fn shader_files(extension: &str) -> Vec<std::path::PathBuf> {
    let mut files: Vec<_> = std::fs::read_dir(shader_dir())
        .expect("the shaders directory")
        .map(|e| e.expect("an entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == extension))
        .collect();
    files.sort();
    files
}

/// Every `.rs` file under a directory, however deep.
fn rust_files(dir: &std::path::Path, found: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("a source directory") {
        let path = entry.expect("an entry").path();
        if path.is_dir() {
            rust_files(&path, found);
        } else if path.extension().is_some_and(|x| x == "rs") {
            found.push(path);
        }
    }
}

/// The shaders are WESL, and a `.wgsl` file is not.
///
/// Bevy 0.20 hands a `.wgsl` file to wgpu verbatim, with no composition
/// at all, so a `naga_oil` directive or a `baylee_client::shaders::…`
/// import left in one fails only when the pipeline is built, on a GPU,
/// long after every test has passed. Nothing here is meant to be plain
/// WGSL; a file with that extension is a shader that was not moved over.
#[test]
fn the_shaders_are_wesl_and_none_is_left_as_wgsl() {
    let wgsl = shader_files("wgsl");
    assert!(
        wgsl.is_empty(),
        "a .wgsl file reaches wgpu uncomposed in bevy 0.20; rename it to .wesl and convert \
             its directives: {wgsl:#?}"
    );
    let wesl = shader_files("wesl").len();
    assert!(wesl >= 20, "only {wesl} .wesl shaders found");
}

/// The code of a shader: its source without `//` comments, which are
/// free to say "#import" (and do, explaining what replaced it).
fn code_of(source: &str) -> String {
    source
        .lines()
        .map(|line| line.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

/// No shader still speaks `naga_oil`.
///
/// WESL has `import`, `@if` and constants where `naga_oil` had `#import`,
/// `#ifdef`, `#define_import_path` and `#{…}` substitution. WESL does
/// not read the old directives: they stay in the source as text that is
/// not WGSL, and the shader fails when its pipeline is built.
#[test]
fn no_shader_still_uses_naga_oil_directives() {
    let mut faults = Vec::new();
    for path in shader_files("wesl") {
        let source = std::fs::read_to_string(&path).expect("a shader");
        for (n, line) in code_of(&source).lines().enumerate() {
            let line = line.trim_start();
            let old = ["#import", "#ifdef", "#define_import_path"]
                .into_iter()
                .any(|directive| line.starts_with(directive))
                || line.contains("#{");
            if old {
                faults.push(format!(
                    "{}:{}: `{line}`",
                    path.file_name().unwrap_or_default().to_string_lossy(),
                    n + 1
                ));
            }
        }
    }
    assert!(
        faults.is_empty(),
        "naga_oil syntax in a WESL shader: {faults:#?}"
    );
}

/// The bevy 0.20 shader modules this client imports, as WESL paths.
const BEVY_MODULES: [&str; 6] = [
    "bevy_pbr::render::forward_io",
    "bevy_pbr::render::mesh_view_bindings",
    "bevy_pbr::render::mesh_functions",
    "bevy_pbr::render::view_transformations",
    "bevy_render::globals",
    "bevy_ui_render::ui_vertex_output",
];

/// Every `import` statement of every shader, as the file it is in and the
/// module it names: the path without the item (or the `{…}` list) it
/// brings in.
fn shader_imports() -> Vec<(String, String)> {
    let mut imports = Vec::new();
    for path in shader_files("wesl") {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let source = std::fs::read_to_string(&path).expect("a shader");
        // A statement runs to its `;`, which may be lines later.
        let code = code_of(&source);
        let mut statement: Option<String> = None;
        for line in code.lines() {
            if statement.is_none() && line.starts_with("import ") {
                statement = Some(String::new());
            }
            if let Some(text) = statement.as_mut() {
                text.push_str(line.trim());
                if line.contains(';') {
                    let full = statement.take().unwrap_or_default();
                    let path_text = full
                        .trim_start_matches("import ")
                        .split(';')
                        .next()
                        .unwrap_or("")
                        .replace(' ', "");
                    let module = match path_text.split_once("::{") {
                        Some((module, _)) => module.to_string(),
                        None => path_text
                            .rsplit_once("::")
                            .map_or(path_text.clone(), |(module, _)| module.to_string()),
                    };
                    imports.push((name.clone(), module));
                }
            }
        }
        assert!(statement.is_none(), "{name}: an import with no `;`");
    }
    imports
}

/// Every `import` in every shader names a module that exists.
///
/// An import of a path that is not there fails when the shader is
/// composed, which is at pipeline build, on a GPU. Ours are checked
/// against the files; bevy's against the list of modules this client
/// uses, taken from the bevy 0.20 sources, so a path that moved in a
/// later bevy is found by this test rather than by a black screen.
#[test]
fn every_shader_import_names_a_module_that_exists() {
    let imports = shader_imports();
    let mut faults = Vec::new();
    for (name, module) in &imports {
        if module.starts_with("baylee_client::shaders::") {
            let target = module.split("::").nth(2).unwrap_or("");
            if !shader_dir().join(format!("{target}.wesl")).is_file() {
                faults.push(format!(
                    "{name}: `{module}` has no src/shaders/{target}.wesl"
                ));
            }
        } else if !BEVY_MODULES.contains(&module.as_str()) {
            faults.push(format!(
                "{name}: `{module}` is not a module this client is known to import; check \
                     the module path against the bevy 0.20 sources (and add it to BEVY_MODULES \
                     if it is right)"
            ));
        }
    }
    assert!(imports.len() >= 30, "only {} imports found", imports.len());
    assert!(faults.is_empty(), "{faults:#?}");
}

/// Every module of ours that a shader imports is registered as a shader
/// library.
///
/// In bevy 0.20 an imported WESL module resolves only if its file was
/// loaded as a shader (`load_shader_library!`); `embedded_asset!` puts
/// the bytes in the binary but registers no module path, so the import
/// fails at pipeline build with "module not found … no shader
/// registered for this module path", on a GPU and never in a test that
/// only parses. The registration is a string in Rust that agrees with
/// the import only by being typed the same, so it is looked for as text
/// in the source tree, the pattern put together here so this file does
/// not find itself.
#[test]
fn every_module_a_shader_imports_is_registered_as_a_shader_library() {
    let mut libraries: Vec<String> = shader_imports()
        .into_iter()
        .filter_map(|(_, module)| {
            module
                .strip_prefix("baylee_client::shaders::")
                .map(|name| name.split("::").next().unwrap_or("").to_string())
        })
        .collect();
    libraries.sort();
    libraries.dedup();
    assert!(
        libraries.len() >= 3,
        "only {libraries:?} imported (noise, card_common and shell_common are)"
    );
    let mut sources = Vec::new();
    rust_files(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut sources,
    );
    let registrations: String = sources
        .iter()
        .map(|path| std::fs::read_to_string(path).expect("a source file"))
        .collect::<Vec<_>>()
        .join("\n");
    let call = ["load_shader_library", "!(app, \"shaders/"].concat();
    let unregistered: Vec<_> = libraries
        .iter()
        .filter(|name| !registrations.contains(&format!("{call}{name}.wesl\")")))
        .collect();
    assert!(
        unregistered.is_empty(),
        "imported by a shader but never registered with load_shader_library: {unregistered:?}"
    );
}

/// Every shader the Rust side names is a file that exists.
///
/// A shader is embedded with `embedded_asset!` (or registered with
/// `load_shader_library!`) and asked for by an `embedded://` path; the two are strings that agree only by being typed
/// the same. A path to a file that was renamed to `.wesl` (or not
/// renamed) is no compile error: the pipeline waits for a shader that
/// never loads and the material draws nothing. The patterns are put
/// together here so this file does not find itself.
#[test]
fn every_shader_the_code_names_is_a_wesl_file_that_exists() {
    let embed = ["embedded_asset", "!("].concat();
    let library = ["load_shader_library", "!("].concat();
    let url = ["\"embedded", "://baylee_client/shaders/"].concat();
    let mut sources = Vec::new();
    rust_files(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut sources,
    );
    let (mut embedded, mut asked) = (0, 0);
    let mut faults = Vec::new();
    for path in sources {
        let source = std::fs::read_to_string(&path).expect("a source file");
        let file = path.file_name().unwrap_or_default().to_string_lossy();
        let mut check = |name: &str, how: &str| {
            let is_wesl = std::path::Path::new(name)
                .extension()
                .is_some_and(|extension| extension == "wesl");
            if !is_wesl || !shader_dir().join(name).is_file() {
                faults.push(format!("{file}: {how} names `{name}`, no .wesl shader"));
            }
        };
        for (pattern, how) in [
            (&embed, "an embedded asset"),
            (&library, "a shader library"),
        ] {
            for (at, _) in source.match_indices(pattern) {
                let call = &source[at + pattern.len()..];
                let Some(open) = call.find('"') else { continue };
                // `app, "shaders/x.wesl"`: the literal is the call's
                // second argument; anything else before the quote is
                // not a call.
                if call[..open].contains(')') {
                    continue;
                }
                let name = call[open + 1..].split('"').next().unwrap_or("");
                embedded += 1;
                check(name.strip_prefix("shaders/").unwrap_or(name), how);
            }
        }
        for (at, _) in source.match_indices(&url) {
            let name = source[at + url.len()..].split('"').next().unwrap_or("");
            asked += 1;
            check(name, "an embedded:// path");
        }
    }
    assert!(
        (20..=80).contains(&(embedded + asked)),
        "{embedded} embedded and {asked} asked-for shaders found"
    );
    assert!(
        embedded >= 10 && asked >= 10,
        "{embedded} embedded and {asked} asked-for shaders found"
    );
    assert!(faults.is_empty(), "{faults:#?}");
}
