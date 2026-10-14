//! Mesh the declarative, original lantern scene without a Blender/Python process.

// Bounded mesh indices and glTF float32 buffers intentionally narrow f64 math.
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use std::path::{Path, PathBuf};

use anyhow::{Result, ensure};
use serde::Deserialize;
use serde_json::Value;

use super::glb;

#[derive(clap::Args)]
pub(super) struct Args {
    /// Declarative scene source, relative to the repository root.
    #[arg(long, default_value = "art/baylee/lantern.json")]
    source: PathBuf,
    /// Candidate GLB; defaults to a temporary path to protect shipped artwork.
    #[arg(long, default_value = "/tmp/baylee-lantern.glb")]
    output: PathBuf,
    /// Replace an existing candidate output.
    #[arg(long)]
    overwrite: bool,
}

#[derive(Deserialize)]
pub(super) struct Scene {
    pub schema: u32,
    pub name: String,
    pub materials: Vec<Material>,
    pub nodes: Vec<Node>,
    pub camera: Camera,
    pub lights: Vec<Value>,
    pub render: Value,
    pub world: Value,
}

#[derive(Deserialize)]
pub(super) struct Material {
    pub name: String,
    pub color: [f64; 3],
    pub metallic: f64,
    pub roughness: f64,
    pub emission: f64,
}

#[derive(Deserialize)]
pub(super) struct Camera {
    pub name: String,
    pub position: [f64; 3],
    pub target: [f64; 3],
    pub orthographic_scale: f64,
}

#[derive(Deserialize)]
pub(super) struct Node {
    pub name: String,
    pub material: usize,
    pub geometry: Geometry,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(super) enum Geometry {
    Cylinder {
        radius: f64,
        depth: f64,
        center: [f64; 3],
        segments: usize,
        bevel: f64,
        bevel_segments: usize,
    },
    Cone {
        radius_bottom: f64,
        radius_top: f64,
        depth: f64,
        center: [f64; 3],
        segments: usize,
    },
    Sphere {
        radius: f64,
        center: [f64; 3],
        segments: usize,
        rings: usize,
    },
    Tube {
        points: Vec<[f64; 3]>,
        radius: f64,
        segments: usize,
        cyclic: bool,
    },
}

#[derive(Default)]
pub(super) struct Mesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
}

type V3 = [f64; 3];
fn add(a: V3, b: V3) -> V3 {
    std::array::from_fn(|i| a[i] + b[i])
}
fn sub(a: V3, b: V3) -> V3 {
    std::array::from_fn(|i| a[i] - b[i])
}
fn scale(a: V3, s: f64) -> V3 {
    a.map(|v| v * s)
}
fn dot(a: V3, b: V3) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn unit(a: V3) -> V3 {
    scale(a, 1.0 / dot(a, a).sqrt())
}
pub(super) fn yup(p: V3) -> V3 {
    [p[0], p[2], -p[1]]
}

impl Mesh {
    fn vertex(&mut self, position: V3, normal: V3) -> u32 {
        let index = self.positions.len() as u32;
        self.positions.push(yup(position).map(|v| v as f32));
        self.normals.push(yup(normal).map(|v| v as f32));
        index
    }
    fn triangle(&mut self, a: u32, b: u32, c: u32) {
        self.indices.extend([a, b, c]);
    }
    fn rings(&mut self, first: usize, count: usize, sides: usize, cyclic: bool) {
        for ring in 0..if cyclic { count } else { count - 1 } {
            let next = (ring + 1) % count;
            for side in 0..sides {
                let a = (first + ring * sides + side) as u32;
                let b = (first + ring * sides + (side + 1) % sides) as u32;
                let c = (first + next * sides + side) as u32;
                let d = (first + next * sides + (side + 1) % sides) as u32;
                self.triangle(a, b, c);
                self.triangle(b, d, c);
            }
        }
    }
}

fn valid_segments(segments: usize) -> Result<()> {
    ensure!(
        (3..=4096).contains(&segments),
        "mesh segments must be 3..4096"
    );
    Ok(())
}

// Profile is bottom-to-top: each (radius, z, radial-normal, axial-normal).
fn lathe(profile: &[[f64; 4]], sides: usize, center: V3) -> Mesh {
    let mut mesh = Mesh::default();
    for &[radius, z, radial, axial] in profile {
        for side in 0..sides {
            let angle = std::f64::consts::TAU * side as f64 / sides as f64;
            mesh.vertex(
                add(center, [radius * angle.cos(), radius * angle.sin(), z]),
                [radial * angle.cos(), radial * angle.sin(), axial],
            );
        }
    }
    mesh.rings(0, profile.len(), sides, false);
    let bottom = mesh.vertex(add(center, [0.0, 0.0, profile[0][1]]), [0.0, 0.0, -1.0]);
    let top_z = profile.last().expect("nonempty lathe profile")[1];
    let top = mesh.vertex(add(center, [0.0, 0.0, top_z]), [0.0, 0.0, 1.0]);
    // Separate caps retain planar normals rather than rounding the whole face.
    let mut caps = Vec::new();
    for (&[radius, z, _, _], normal) in [
        (&profile[0], [0.0, 0.0, -1.0]),
        (profile.last().expect("profile"), [0.0, 0.0, 1.0]),
    ] {
        let mut ring = Vec::new();
        for side in 0..sides {
            let angle = std::f64::consts::TAU * side as f64 / sides as f64;
            ring.push(mesh.vertex(
                add(center, [radius * angle.cos(), radius * angle.sin(), z]),
                normal,
            ));
        }
        caps.push(ring);
    }
    for side in 0..sides {
        let next = (side + 1) % sides;
        mesh.triangle(bottom, caps[0][next], caps[0][side]);
        mesh.triangle(top, caps[1][side], caps[1][next]);
    }
    mesh
}

fn cylinder(
    radius: f64,
    depth: f64,
    center: V3,
    sides: usize,
    bevel: f64,
    steps: usize,
) -> Result<Mesh> {
    valid_segments(sides)?;
    ensure!(
        radius > 0.0 && depth > 0.0 && bevel >= 0.0 && steps > 0 && steps <= 64,
        "invalid cylinder dimensions"
    );
    let b = bevel.min(depth / 2.0).min(radius / 2.0);
    if b == 0.0 {
        return Ok(lathe(
            &[
                [radius, -depth / 2.0, 1.0, 0.0],
                [radius, depth / 2.0, 1.0, 0.0],
            ],
            sides,
            center,
        ));
    }
    let mut profile = Vec::new();
    for step in 0..=steps {
        let angle =
            -std::f64::consts::FRAC_PI_2 + std::f64::consts::FRAC_PI_2 * step as f64 / steps as f64;
        profile.push([
            radius - b + b * angle.cos(),
            -depth / 2.0 + b + b * angle.sin(),
            angle.cos(),
            angle.sin(),
        ]);
    }
    for step in 0..=steps {
        let angle = std::f64::consts::FRAC_PI_2 * step as f64 / steps as f64;
        profile.push([
            radius - b + b * angle.cos(),
            depth / 2.0 - b + b * angle.sin(),
            angle.cos(),
            angle.sin(),
        ]);
    }
    Ok(lathe(&profile, sides, center))
}

fn sphere(radius: f64, center: V3, sides: usize, rings: usize) -> Result<Mesh> {
    valid_segments(sides)?;
    ensure!(
        radius > 0.0 && (3..=4096).contains(&rings),
        "invalid sphere dimensions"
    );
    let mut mesh = Mesh::default();
    for ring in 1..rings {
        let latitude =
            -std::f64::consts::FRAC_PI_2 + std::f64::consts::PI * ring as f64 / rings as f64;
        for side in 0..sides {
            let longitude = std::f64::consts::TAU * side as f64 / sides as f64;
            let normal = [
                latitude.cos() * longitude.cos(),
                latitude.cos() * longitude.sin(),
                latitude.sin(),
            ];
            mesh.vertex(add(center, scale(normal, radius)), normal);
        }
    }
    mesh.rings(0, rings - 1, sides, false);
    let bottom = mesh.vertex(add(center, [0.0, 0.0, -radius]), [0.0, 0.0, -1.0]);
    let top = mesh.vertex(add(center, [0.0, 0.0, radius]), [0.0, 0.0, 1.0]);
    for side in 0..sides {
        let next = (side + 1) % sides;
        mesh.triangle(bottom, next as u32, side as u32);
        mesh.triangle(
            top,
            ((rings - 2) * sides + side) as u32,
            ((rings - 2) * sides + next) as u32,
        );
    }
    Ok(mesh)
}

// A polyline tube meets on the bisector plane. Stretch only the radial
// component in the bend plane so each adjacent cylindrical segment keeps
// its radius; otherwise sharp corners fold back through their own surface.
fn join_frame(previous: V3, point: V3, next: V3) -> Result<(V3, V3, f64)> {
    let mut incoming = sub(point, previous);
    let mut outgoing = sub(next, point);
    if dot(incoming, incoming) < 1e-24 {
        incoming = outgoing;
    }
    if dot(outgoing, outgoing) < 1e-24 {
        outgoing = incoming;
    }
    ensure!(
        dot(incoming, incoming) > 1e-24 && dot(outgoing, outgoing) > 1e-24,
        "zero-length tube tangent"
    );
    let incoming = unit(incoming);
    let outgoing = unit(outgoing);
    let cosine = dot(incoming, outgoing).clamp(-1.0, 1.0);
    let half_cosine = 1.0_f64.midpoint(cosine).sqrt();
    ensure!(half_cosine > 1e-6, "tube reverses direction at a joint");
    let bend = sub(outgoing, incoming);
    let denominator = dot(bend, bend);
    let stretch = if denominator > 1e-24 {
        (1.0 / half_cosine - 1.0) / denominator
    } else {
        0.0
    };
    Ok((unit(add(incoming, outgoing)), bend, stretch))
}

// Shortest-rotation parallel transport, including a stable half-turn.
fn transport(previous: Option<(V3, V3)>, tangent: V3) -> V3 {
    previous.map_or_else(
        || {
            if tangent[2].abs() < 0.9 {
                [0.0, 0.0, 1.0]
            } else {
                [1.0, 0.0, 0.0]
            }
        },
        |(previous, normal)| {
            let axis = cross(previous, tangent);
            let cosine = dot(previous, tangent).clamp(-1.0, 1.0);
            if cosine > -1.0 + 1e-12 {
                add(
                    add(normal, cross(axis, normal)),
                    scale(cross(axis, cross(axis, normal)), 1.0 / (1.0 + cosine)),
                )
            } else {
                normal
            }
        },
    )
}

fn tube(points: &[V3], radius: f64, sides: usize, cyclic: bool) -> Result<Mesh> {
    valid_segments(sides)?;
    ensure!(
        points.len() >= 2 && points.len() <= 100_000 && radius > 0.0,
        "invalid tube dimensions"
    );
    ensure!(
        points.iter().flatten().all(|x| x.is_finite()),
        "non-finite tube path"
    );
    let mut points = points.to_vec();
    // Blender's diamond is a closed path whose last point repeats the first.
    // Remove the repeat and connect its rings to avoid a zero-length tangent.
    let closed = cyclic
        || dot(
            sub(points[0], *points.last().expect("points")),
            sub(points[0], *points.last().expect("points")),
        ) < 1e-24;
    if closed
        && dot(
            sub(points[0], *points.last().expect("points")),
            sub(points[0], *points.last().expect("points")),
        ) < 1e-24
    {
        points.pop();
    }
    ensure!(points.len() >= 2, "tube has fewer than two unique points");
    let mut mesh = Mesh::default();
    let mut last_frame: Option<(V3, V3)> = None;
    for (i, &point) in points.iter().enumerate() {
        let previous = if i > 0 {
            points[i - 1]
        } else if closed {
            points[points.len() - 1]
        } else {
            point
        };
        let next = if i + 1 < points.len() {
            points[i + 1]
        } else if closed {
            points[0]
        } else {
            point
        };
        let (tangent, bend, stretch) = join_frame(previous, point, next)?;
        let seed = transport(last_frame, tangent);
        let projected = sub(seed, scale(tangent, dot(seed, tangent)));
        ensure!(dot(projected, projected) > 1e-24, "degenerate tube frame");
        let normal = unit(projected);
        let binormal = cross(tangent, normal);
        last_frame = Some((tangent, normal));
        for side in 0..sides {
            let angle = std::f64::consts::TAU * side as f64 / sides as f64;
            let radial = add(scale(normal, angle.cos()), scale(binormal, angle.sin()));
            let offset = add(radial, scale(bend, dot(radial, bend) * stretch));
            mesh.vertex(add(point, scale(offset, radius)), radial);
        }
    }
    mesh.rings(0, points.len(), sides, closed);
    // POLY curves in the original have use_fill_caps=false: open tube ends.
    Ok(mesh)
}

pub(super) fn meshes(scene: &Scene) -> Result<Vec<Mesh>> {
    ensure!(
        scene.schema == 1 && !scene.nodes.is_empty(),
        "unsupported or empty lantern scene"
    );
    scene
        .nodes
        .iter()
        .map(|node| {
            ensure!(
                node.material < scene.materials.len(),
                "{}: unknown material",
                node.name
            );
            let mesh = match &node.geometry {
                Geometry::Cylinder {
                    radius,
                    depth,
                    center,
                    segments,
                    bevel,
                    bevel_segments,
                } => cylinder(*radius, *depth, *center, *segments, *bevel, *bevel_segments)?,
                Geometry::Cone {
                    radius_bottom,
                    radius_top,
                    depth,
                    center,
                    segments,
                } => {
                    valid_segments(*segments)?;
                    ensure!(
                        *radius_bottom > 0.0 && *radius_top > 0.0 && *depth > 0.0,
                        "invalid cone"
                    );
                    let slope = (radius_bottom - radius_top) / depth;
                    let radial = 1.0 / (1.0 + slope * slope).sqrt();
                    lathe(
                        &[
                            [*radius_bottom, -depth / 2.0, radial, slope * radial],
                            [*radius_top, depth / 2.0, radial, slope * radial],
                        ],
                        *segments,
                        *center,
                    )
                }
                Geometry::Sphere {
                    radius,
                    center,
                    segments,
                    rings,
                } => sphere(*radius, *center, *segments, *rings)?,
                Geometry::Tube {
                    points,
                    radius,
                    segments,
                    cyclic,
                } => tube(points, *radius, *segments, *cyclic)?,
            };
            ensure!(
                mesh.positions.iter().flatten().all(|x| x.is_finite()),
                "{}: non-finite mesh",
                node.name
            );
            Ok(mesh)
        })
        .collect()
}

pub(super) fn camera_matrix(camera: &Camera) -> Result<[f64; 16]> {
    let eye = yup(camera.position);
    let back = sub(eye, yup(camera.target));
    ensure!(dot(back, back) > 1e-24, "camera position equals target");
    let back = unit(back);
    let right = cross([0.0, 1.0, 0.0], back);
    ensure!(dot(right, right) > 1e-24, "camera points along its up axis");
    let right = unit(right);
    let up = cross(back, right);
    Ok([
        right[0], right[1], right[2], 0.0, up[0], up[1], up[2], 0.0, back[0], back[1], back[2],
        0.0, eye[0], eye[1], eye[2], 1.0,
    ])
}

pub(super) fn run(root: &Path, args: &Args) -> Result<()> {
    ensure!(
        args.output
            .extension()
            .is_some_and(|extension| extension == "glb"),
        "output must be a .glb file"
    );
    ensure!(
        args.overwrite || !args.output.exists(),
        "{} exists; use --overwrite to replace it",
        args.output.display()
    );
    let scene: Scene = serde_json::from_slice(&std::fs::read(root.join(&args.source))?)?;
    let bytes = glb::encode(&scene, &meshes(&scene)?)?;
    super::write_file(&args.output, &bytes)?;
    println!(
        "{}: {} meshes, {} materials, {} bytes",
        args.output.display(),
        scene.nodes.len(),
        scene.materials.len(),
        bytes.len()
    );
    println!(
        "Camera exported; original disk lights, world and Cycles settings retained in extras.baylee. Existing .blend and PNG are authored separately."
    );
    Ok(())
}

#[cfg(test)]
// Exact equality is intentional for seam samples and authored JSON constants.
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    pub(super) fn original_scene() -> Scene {
        serde_json::from_str(include_str!("../../../../art/baylee/lantern.json")).unwrap()
    }

    #[test]
    fn original_objects_and_materials_keep_their_geometry() {
        let scene = original_scene();
        assert_eq!(scene.nodes.len(), 50);
        assert_eq!(scene.materials.len(), 4);
        assert_eq!(scene.lights.len(), 3);
        let meshes = meshes(&scene).unwrap();
        let mut bounds = [[f32::INFINITY; 3], [f32::NEG_INFINITY; 3]];
        for (mesh_index, mesh) in meshes.iter().enumerate() {
            assert_eq!(mesh.positions.len(), mesh.normals.len());
            assert!(mesh.indices.len().is_multiple_of(3));
            for (p, n) in mesh.positions.iter().zip(&mesh.normals) {
                for axis in 0..3 {
                    bounds[0][axis] = bounds[0][axis].min(p[axis]);
                    bounds[1][axis] = bounds[1][axis].max(p[axis]);
                }
                assert!((n.iter().map(|n| n * n).sum::<f32>() - 1.0).abs() < 1e-5);
            }
            for triangle in mesh.indices.as_chunks::<3>().0 {
                let [a, b, c] =
                    std::array::from_fn(|i| mesh.positions[triangle[i] as usize].map(f64::from));
                let normal = cross(sub(b, a), sub(c, a));
                assert!(dot(normal, normal) > 1e-20, "degenerate triangle");
                let authored = mesh.normals[triangle[0] as usize].map(f64::from);
                assert!(
                    dot(normal, authored) > 0.0,
                    "inward triangle on {}: {triangle:?}, positions {a:?} {b:?} {c:?}, normal {authored:?}",
                    scene.nodes[mesh_index].name
                );
            }
        }
        assert!((bounds[0][0] + 0.41).abs() < 1e-5 && (bounds[1][0] - 0.41).abs() < 1e-5);
        assert!(bounds[0][1].abs() < 1e-5 && (bounds[1][1] - 1.728).abs() < 0.002);
        assert_eq!(scene.materials[3].emission, 2.2);
    }

    #[test]
    fn camera_aims_local_negative_z_at_original_target() {
        let scene = original_scene();
        let matrix = camera_matrix(&scene.camera).unwrap();
        let back = [matrix[8], matrix[9], matrix[10]];
        let direction = unit(sub(yup(scene.camera.target), yup(scene.camera.position)));
        assert!((dot(back, direction) + 1.0).abs() < 1e-12);
        assert_eq!(&matrix[12..15], &yup([2.8, -6.0, 2.8]));
    }
}
