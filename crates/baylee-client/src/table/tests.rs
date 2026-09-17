use super::*;

fn slot(facing: f32) -> SeatSlot {
    SeatSlot {
        player: baylee_core::ids::PlayerId::new(0),
        ring_index: 0,
        angle: facing,
        center: Vec2::ZERO,
        facing,
        half_extent: Vec2::new(6.0, 3.0),
        reclaimed: 0.0,
        is_local: true,
    }
}

/// Every vertex of the card mesh, as (x, y, z).
fn points(mesh: &Mesh) -> Vec<(f32, f32, f32)> {
    let Some(bevy::mesh::VertexAttributeValues::Float32x3(p)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        panic!("card mesh has positions")
    };
    p.iter().map(|v| (v[0], v[1], v[2])).collect()
}

/// The outline of the printed face, in order, as (x, y) pairs.
///
/// Vertex 0 is the fan's centre and is not part of the outline; the wall
/// vertices that follow the face are told apart by their normal, which is
/// the thing that actually distinguishes them.
fn rim(mesh: &Mesh) -> Vec<(f32, f32)> {
    let Some(bevy::mesh::VertexAttributeValues::Float32x3(n)) =
        mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
    else {
        panic!("card mesh has normals")
    };
    let face = n.iter().take_while(|v| v[2] > 0.5).count();
    points(mesh)[1..face].iter().map(|v| (v.0, v.1)).collect()
}

/// Triangles as index triples.
fn triangles(mesh: &Mesh) -> Vec<[u32; 3]> {
    let Some(Indices::U32(idx)) = mesh.indices() else {
        panic!("card mesh is indexed")
    };
    idx.as_chunks::<3>().0.to_vec()
}

/// Twice the signed area of a triangle, positive when it is wound
/// counter-clockwise seen from +z.
fn cross(a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> f32 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

// The card that shipped as a bowtie: every corner arc swept the quarter
// turn belonging to its neighbour, so the outline crossed itself twice
// through the middle and a permanent on the battlefield was drawn as a
// small bright X. `an_untapped_card_lies_flat_on_the_table` passed the
// whole time — the transform was never the problem — so the mesh needs
// tests of its own.
#[test]
fn the_card_outline_never_folds_through_its_own_middle() {
    let mesh = rounded_card_mesh(CARD_WIDTH, CARD_HEIGHT, CARD_CORNER);
    let rim = rim(&mesh);
    // Walking a convex outline turns the same way at every vertex and
    // comes back around exactly once. A bowtie turns back on itself.
    let mut turn = 0.0_f32;
    for i in 0..rim.len() {
        let (a, b, c) = (rim[i], rim[(i + 1) % rim.len()], rim[(i + 2) % rim.len()]);
        assert!(
            cross(a, b, c) >= 0.0,
            "the outline turns back on itself at vertex {i}: {a:?} {b:?} {c:?}"
        );
        let before = (b.1 - a.1).atan2(b.0 - a.0);
        let after = (c.1 - b.1).atan2(c.0 - b.0);
        let mut delta = after - before;
        while delta > std::f32::consts::PI {
            delta -= std::f32::consts::TAU;
        }
        while delta < -std::f32::consts::PI {
            delta += std::f32::consts::TAU;
        }
        turn += delta;
    }
    assert!(
        (turn - std::f32::consts::TAU).abs() < 1e-3,
        "a closed convex outline turns through exactly one full circle, not {turn}"
    );
}

#[test]
fn the_card_mesh_covers_the_card() {
    let mesh = rounded_card_mesh(CARD_WIDTH, CARD_HEIGHT, CARD_CORNER);
    let rim = rim(&mesh);
    let area: f32 = (0..rim.len())
        .map(|i| {
            let (a, b) = (rim[i], rim[(i + 1) % rim.len()]);
            a.0.mul_add(b.1, -(b.0 * a.1))
        })
        .sum::<f32>()
        / 2.0;
    // The rounded rectangle, minus what the four corner arcs cut away.
    // The arcs are drawn in segments, so the mesh is a hair under.
    let ideal = CARD_WIDTH * CARD_HEIGHT - (4.0 - std::f32::consts::PI) * CARD_CORNER * CARD_CORNER;
    assert!(
        area > ideal * 0.99 && area <= ideal,
        "a card of {CARD_WIDTH}×{CARD_HEIGHT} covers about {ideal}, not {area}"
    );
    // And it stays inside the card: no vertex may stick out past an edge.
    for (x, y) in rim {
        assert!(
            x.abs() <= CARD_WIDTH / 2.0 + 1e-5 && y.abs() <= CARD_HEIGHT / 2.0 + 1e-5,
            "({x}, {y}) is outside the card"
        );
    }
}

#[test]
fn every_face_triangle_faces_the_printed_side() {
    let mesh = rounded_card_mesh(CARD_WIDTH, CARD_HEIGHT, CARD_CORNER);
    let points = points(&mesh);
    let outline = rim(&mesh).len();
    // Back-face culling is on, so a triangle wound the other way is an
    // invisible sliver of card.
    for tri in triangles(&mesh) {
        if tri.iter().any(|i| *i as usize > outline) {
            continue; // a wall triangle; checked below
        }
        let flat = |i: u32| (points[i as usize].0, points[i as usize].1);
        let (a, b, c) = (flat(tri[0]), flat(tri[1]), flat(tri[2]));
        assert!(
            cross(a, b, c) > 0.0,
            "face triangle {tri:?} faces away from the camera"
        );
    }
}

/// A card is a slab, not a decal: it has a wall around its edge so it
/// reads as lying *on* the table. Wound the other way, that wall is a
/// card you can see straight through from the side.
#[test]
fn the_card_wall_faces_outwards_and_stands_on_the_table() {
    let mesh = rounded_card_mesh(CARD_WIDTH, CARD_HEIGHT, CARD_CORNER);
    let points = points(&mesh);
    let outline = rim(&mesh).len();
    let mut walls = 0;
    for tri in triangles(&mesh) {
        if tri.iter().all(|i| *i as usize <= outline) {
            continue;
        }
        walls += 1;
        let corner = |i: u32| points[i as usize];
        let (first, second, third) = (corner(tri[0]), corner(tri[1]), corner(tri[2]));
        // The triangle's normal, and the direction away from the card's
        // axis at its centroid. A wall faces out when they agree.
        let edge_a = (second.0 - first.0, second.1 - first.1, second.2 - first.2);
        let edge_b = (third.0 - first.0, third.1 - first.1, third.2 - first.2);
        let normal = (
            edge_a.1 * edge_b.2 - edge_a.2 * edge_b.1,
            edge_a.2 * edge_b.0 - edge_a.0 * edge_b.2,
            edge_a.0 * edge_b.1 - edge_a.1 * edge_b.0,
        );
        let out = (
            (first.0 + second.0 + third.0) / 3.0,
            (first.1 + second.1 + third.1) / 3.0,
        );
        assert!(
            normal.0.mul_add(out.0, normal.1 * out.1) > 0.0,
            "wall triangle {tri:?} faces into the card"
        );
        for point in [first, second, third] {
            assert!(
                point.2 >= -1e-6 && point.2 <= CARD_THICKNESS + 1e-6,
                "the wall runs past the card's own thickness at {point:?}"
            );
        }
    }
    assert_eq!(
        walls,
        outline * 2,
        "the wall does not close around the card"
    );
}

/// The printed face is what a player looks at, and it has to be the
/// topmost surface — a face level with the wall would z-fight along every
/// edge of every card on the table.
#[test]
fn the_printed_face_sits_on_top_of_the_slab() {
    let mesh = rounded_card_mesh(CARD_WIDTH, CARD_HEIGHT, CARD_CORNER);
    let points = points(&mesh);
    let outline = rim(&mesh).len();
    for point in &points[..=outline] {
        assert!(
            (point.2 - CARD_THICKNESS).abs() < 1e-6,
            "a face vertex is not on top: {point:?}"
        );
    }
    assert!(
        points.iter().any(|p| p.2.abs() < 1e-6),
        "nothing touches the table, so the card floats"
    );
}

/// The table is a slab with a thickness, and its surface is the plane
/// everything on the stage is placed against.
///
/// Both halves matter and the second is the one that would break the
/// board silently: the body hangs *below* zero. Build it standing on zero
/// instead and every mat, glow, medallion and card is inside the table,
/// which no test about transforms would notice — the transforms would all
/// still be right.
#[test]
fn the_table_is_a_slab_and_not_a_sheet() {
    let span = Vec2::new(34.0, 26.0);
    let mesh = slab_mesh(span);
    let points = points(&mesh);
    let high = points.iter().fold(f32::NEG_INFINITY, |a, p| a.max(p.2));
    let low = points.iter().fold(f32::INFINITY, |a, p| a.min(p.2));
    assert!(
        high.abs() < 1e-6,
        "the table's surface is not at zero: {high}"
    );
    assert!(
        (low + TABLE_THICKNESS).abs() < 1e-6,
        "the table is {} deep, not {TABLE_THICKNESS}",
        -low
    );
}

/// And the table stops before the window does, so there is a sky to see.
///
/// Measured against the window rather than against the table. The camera
/// frames the layout plus [`AIR`] and the slab is cut to the layout plus
/// [`SLAB_MARGIN`], so every point of the table's rim has to stand inside
/// the framed box by the difference. This is the whole reason anything
/// behind the table is ever visible, and it is one subtraction away from
/// being false again — `SLAB_MARGIN` is derived from a rail width that a
/// later change to the table's look could quietly grow.
#[test]
fn the_table_stops_before_the_window_does() {
    let span = Vec2::new(34.0, 26.0);
    let rim = rim(&slab_mesh(span));
    let framed = span * 0.5 + Vec2::splat(AIR - SLAB_MARGIN);
    let band = AIR - SLAB_MARGIN;
    assert!(
        band > 0.0,
        "the slab is cut wider than the shot that frames it"
    );
    for &(x, y) in &rim {
        assert!(
            x.abs() <= framed.x - band + 1e-3 && y.abs() <= framed.y - band + 1e-3,
            "the table reaches ({x}, {y}), inside a frame of {framed:?}"
        );
    }
    // And the corner is a corner rather than a bite out of the play area.
    let bite = tabletop::table_corner(span) * (1.0 - std::f32::consts::FRAC_1_SQRT_2);
    assert!(
        bite < SLAB_MARGIN,
        "the corner takes {bite} out of each end, which is play area"
    );
}

#[test]
fn the_card_face_is_mapped_corner_to_corner() {
    let mesh = rounded_card_mesh(CARD_WIDTH, CARD_HEIGHT, CARD_CORNER);
    let Some(bevy::mesh::VertexAttributeValues::Float32x2(uvs)) =
        mesh.attribute(Mesh::ATTRIBUTE_UV_0)
    else {
        panic!("card mesh has uvs")
    };
    let Some(bevy::mesh::VertexAttributeValues::Float32x3(pos)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        panic!("card mesh has positions")
    };
    // The printed face runs left→right and top→bottom, so the top-left
    // of the card is (0,0) in the image and the bottom-right is (1,1).
    for (p, uv) in pos.iter().zip(uvs) {
        let want_u = f32::midpoint(p[0] / (CARD_WIDTH / 2.0), 1.0);
        let want_v = (1.0 - p[1] / (CARD_HEIGHT / 2.0)) * 0.5;
        assert!((uv[0] - want_u).abs() < 1e-5 && (uv[1] - want_v).abs() < 1e-5);
        assert!((0.0..=1.0).contains(&uv[0]) && (0.0..=1.0).contains(&uv[1]));
    }
}

/// The geometry and the two shaders have to round the card at the same
/// radius, and nothing in the compiler can notice that they do: one is a
/// Rust constant and the others are text in a `.wgsl` file. So the text
/// is read.
///
/// Cut the mesh wider than the print and the shader's ink has nothing
/// left to reach; cut it narrower and a white sliver of the scanner bed
/// survives outside it. Either way the card stops looking like a card,
/// which is the entire point of cutting the corner at all.
#[test]
fn the_mesh_and_the_shaders_round_the_card_alike() {
    let printed = crate::cardmat::tests::wgsl_const(
        include_str!("../shaders/card_common.wgsl"),
        "PRINTED_CORNER",
    );
    assert!(
        (CARD_CORNER / CARD_WIDTH - printed).abs() < 1e-6,
        "the mesh rounds at {} of its width, the shaders at {printed}",
        CARD_CORNER / CARD_WIDTH
    );
}

#[test]
fn table_space_maps_away_from_the_seat_into_the_screen() {
    // +y in table space is away from the local seat, which is -z in the
    // world because the camera sits on +z.
    let world = to_world(Vec2::new(2.0, 5.0), 0.0);
    assert!((world.x - 2.0).abs() < 1e-5);
    assert!((world.z + 5.0).abs() < 1e-5);
}

#[test]
fn an_untapped_card_lies_flat_on_the_table() {
    let t = card_transform(&slot(0.0), Vec2::ZERO, false, 0.0);
    // The quad's normal (+z in local space) should point straight up.
    let normal = t.rotation * Vec3::Z;
    assert!((normal.y - 1.0).abs() < 1e-4, "normal was {normal:?}");
}

#[test]
fn tapping_rotates_a_quarter_turn_but_keeps_the_card_on_the_table() {
    let untapped = card_transform(&slot(0.0), Vec2::ZERO, false, 0.0);
    let tapped = card_transform(&slot(0.0), Vec2::ZERO, true, 0.0);
    assert_ne!(untapped.rotation, tapped.rotation);

    // Still flat: only the in-plane orientation changed.
    let normal = tapped.rotation * Vec3::Z;
    assert!((normal.y - 1.0).abs() < 1e-4, "normal was {normal:?}");

    // The card's long axis has swung to the side.
    let up = tapped.rotation * Vec3::Y;
    assert!(
        up.y.abs() < 1e-4,
        "long axis should now lie across the table"
    );
}

/// The material every hidden card wears is built before the printed back
/// has been fetched and dressed in it afterwards, in place — so what
/// dressing produces has to be the material the picture would have been
/// built into. The half that is easy to forget is `has_art`: it follows
/// the handle and not the look, and a material carrying the back's
/// texture with the flag still at zero goes on drawing the flat colour.
#[test]
fn a_dressed_back_is_the_material_the_picture_would_have_built() {
    let picture = Handle::<Image>::default();
    let look = CardLook::flat(BACK_COLOR, FinishTreatment::Plain, 0);

    let mut dressed = material(look, None, BACK_COLOR, MOVING);
    assert!(dressed.art.is_none() && dressed.params.has_art == 0.0);
    dress_in_the_back(&mut dressed, picture.clone());

    let built = material(look, Some(picture), BACK_COLOR, MOVING);
    assert_eq!(dressed.art, built.art);
    assert_eq!(
        format!("{:?}", dressed.params),
        format!("{:?}", built.params),
        "dressing changed something the builder would not have"
    );
}

#[test]
fn cards_float_above_the_felt_so_they_never_z_fight() {
    let t = card_transform(&slot(0.0), Vec2::ZERO, false, 0.0);
    assert!(t.translation.y > TABLE_Y);
}

#[test]
fn a_seat_across_the_table_has_its_cards_turned_to_face_it() {
    let near = card_transform(&slot(0.0), Vec2::ZERO, false, 0.0);
    let far = card_transform(&slot(std::f32::consts::PI), Vec2::ZERO, false, 0.0);
    // Both flat, but their in-plane orientation is opposite.
    let near_up = near.rotation * Vec3::Y;
    let far_up = far.rotation * Vec3::Y;
    assert!(
        near_up.dot(far_up) < -0.9,
        "far seat should be turned around"
    );
}

#[test]
fn only_counted_groups_get_a_badge() {
    use baylee_client_core::board::{KeywordBadge, Provenance};
    use baylee_view::ObjectStatus;

    let mut group = CardGroup {
        representative: ObjectId::new(1, 0),
        members: vec![ObjectId::new(1, 0)],
        name: "Soldier".into(),
        power: Some(1),
        toughness: Some(1),
        base_power: Some(1),
        base_toughness: Some(1),
        damage: 0,
        loyalty: None,
        status: ObjectStatus::NONE,
        counters: vec![],
        badges: Vec::<KeywordBadge>::new(),
        art: None,
        provenance: Provenance::Token,
        original: None,
        summoning_sick: false,
        activatable: false,
        commander: false,
        individual: None,
    };
    assert_eq!(stack_badge(&group), None);
    group.members.push(ObjectId::new(2, 0));
    group.members.push(ObjectId::new(3, 0));
    assert_eq!(stack_badge(&group).as_deref(), Some("×3"));
}
/// The finish is a property of the *printing*, and the print table is per
/// seat: a printing this seat has not earned reads as plain rather than
/// as a hole in the hidden-information rule. This pins the lookup that
/// makes that true, because the alternative — reading a finish off the
/// card — would be the leak.
#[test]
fn a_printing_a_seat_has_not_earned_is_drawn_plain() {
    use baylee_client_core::images::ArtSize;
    use baylee_core::ids::PrintRef;

    let statics = baylee_view::GameStatic {
        view_version: baylee_view::VIEW_VERSION,
        game_id: String::new(),
        your_seat: baylee_core::ids::PlayerId::new(0),
        seats: Vec::new(),
        prints: vec![
            Some(baylee_view::PrintEntry {
                scryfall_id: "11111111-2222-3333-4444-555555555555".to_string(),
                lang: "en".to_string(),
                finish: baylee_view::Finish::Foil,
            }),
            // Earned by nobody: the seat has not seen this card.
            None,
        ],
    };
    let look = |slot: u16| {
        let key = ImageKey::new(PrintRef(slot), 0, ArtSize::Normal);
        key.printing()
            .and_then(|p| statics.print(p))
            .map_or(FinishTreatment::Plain, |entry| entry.finish.into())
    };
    assert_eq!(look(0), FinishTreatment::Foil, "its own deck's printing");
    assert_eq!(look(1), FinishTreatment::Plain, "a hole is not a foil");
}
