use super::*;
use baylee_account::advent::framing::{Framing as Guide, MatVariant, MatVariants};
use baylee_client_core::{
    layout::{Arrangement, Seat, TableLayout},
    tabletop::{LEDGE_FRAC, MAT_CORNER, MAT_MARGIN},
};
use baylee_core::ids::PlayerId;

fn guide(focus: [u16; 2], safe: [u16; 4]) -> Guide {
    Guide {
        focus,
        safe: Some(safe),
    }
}

fn metadata() -> MatFraming {
    let duel = guide([5000, 5000], [1000, 1500, 9000, 8500]);
    let variants = [
        (4, 2, guide([3300, 3300], [700, 1500, 6000, 5900])),
        (5, 3, guide([8000, 5600], [6500, 3000, 9800, 8100])),
        (5, 2, guide([2100, 3000], [200, 1300, 4000, 4800])),
    ]
    .map(|(seats, allies, framing)| MatVariant {
        seats,
        allies,
        sides: 2,
        framing,
    });
    MatFraming {
        duel,
        variants: MatVariants::try_from(variants.as_slice()).unwrap(),
    }
}

fn seats(teams: &[Option<u8>]) -> Vec<Seat> {
    (0u8..)
        .zip(teams)
        .map(|(id, &team)| Seat::on(PlayerId::new(id), team))
        .collect()
}

fn assert_window(f: Framing, viewport: Vec4, field: Vec2, pixels: Vec2) {
    let origin = Vec2::new(viewport.x, viewport.y);
    let span = Vec2::new(viewport.z, viewport.w);
    let stretch = field / (span * pixels);
    assert!((stretch.x / stretch.y - 1.0).abs() < 1e-5);
    assert!((origin - f.protected.min).max_element() < 1e-5);
    assert!((f.protected.max - (origin + span)).max_element() < 1e-5);
    for corner in [
        f.protected.min,
        f.protected.max,
        Vec2::new(f.protected.min.x, f.protected.max.y),
        Vec2::new(f.protected.max.x, f.protected.min.y),
    ] {
        let local = ((corner - origin) / span - Vec2::splat(0.5)) * field;
        let q = local.abs() - field * 0.5 + Vec2::splat(MAT_CORNER);
        let distance = q.max(Vec2::ZERO).length() + q.max_element().min(0.0) - MAT_CORNER;
        assert!(
            distance < 1e-4,
            "protected corner outside the rounded border"
        );
    }
}

#[test]
fn duel_team_and_free_for_all_use_each_actual_field_after_shelf_and_command_space() {
    let meta = metadata();
    let cases = [
        (vec![None, None], vec![5000u16, 5000]),
        (vec![Some(8), Some(3), Some(8), Some(3)], vec![3300; 4]),
        (
            vec![Some(8), Some(3), Some(8), Some(3), Some(8)],
            vec![8000, 2100, 8000, 2100, 8000],
        ),
        (vec![None; 5], vec![5000; 5]),
    ];
    let windows = [
        Vec2::new(1920., 1080.),
        Vec2::new(390., 844.),
        Vec2::new(810., 390.),
    ];
    let pixels = Vec2::new(1536., 864.);
    for (teams, expected_focus) in cases {
        let seats = seats(&teams);
        let roster = Roster::new(seats.iter().map(|s| (s.player, s.team)));
        for window in windows {
            for arrangement in [
                Arrangement::Ring,
                Arrangement::Turntable,
                Arrangement::FocusRing,
            ] {
                let canvas =
                    crate::table::Canvas::for_table(window, arrangement).with_drawer(false);
                let table = TableLayout::arranged_in(
                    &seats,
                    canvas.aspect(),
                    arrangement,
                    None,
                    baylee_client_core::tableview::TableFrame::of(window.x, window.y),
                );
                for mut seat in table.slots.into_iter().filter(|s| !s.parked) {
                    let owner = seat.player.get();
                    let authored = roster.framing(owner, Some(&meta));
                    let expected = f32::from(expected_focus[usize::from(owner)]) / 10000.;
                    assert!((authored.focus.x - expected).abs() < 1e-5);
                    for reclaim in [false, true] {
                        if reclaim {
                            seat.reclaim_command_strip();
                        }
                        let quad = seat.reach() * 2. + Vec2::splat(2. * MAT_MARGIN);
                        let field = quad * Vec2::new(1., 1. - LEDGE_FRAC);
                        for guide in [authored, roster.framing(owner, None)] {
                            assert_window(
                                guide,
                                guide.viewport(quad, pixels, MAT_CORNER),
                                field,
                                pixels,
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn centered_fallback_keeps_every_pixel_and_focal_cover_clamps_at_a_source_edge() {
    let pixels = Vec2::new(1536., 864.);
    let quad = Vec2::new(300., 1000. / (1. - LEDGE_FRAC));
    let fallback = Framing::default().viewport(quad, pixels, MAT_CORNER);
    assert!(fallback.x <= 0. && fallback.x + fallback.z >= 1.);
    assert!((fallback.y + fallback.w * 0.5 - 0.5).abs() < 1e-5);
    let guide = MatFraming {
        duel: Guide {
            focus: [9900, 7000],
            safe: None,
        },
        variants: MatVariants::default(),
    };
    let authored = Roster::default()
        .framing(0, Some(&guide))
        .viewport(quad, pixels, MAT_CORNER);
    assert!((authored.x + authored.z - 1.).abs() < 1e-5);
    assert!(authored.y.abs() < 1e-5 && (authored.w - 1.).abs() < 1e-5);
}

#[test]
fn missing_roster_inherits_the_duel_and_unusable_guidance_keeps_the_original() {
    let mut meta = metadata();
    let empty = Roster::default();
    assert_eq!(empty.framing(6, Some(&meta)).focus, Vec2::splat(0.5));
    meta.duel.safe = Some([9000, 100, 100, 8000]);
    assert_eq!(empty.framing(6, Some(&meta)), Framing::default());
}
