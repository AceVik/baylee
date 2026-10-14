//! Minimal glTF 2.0/GLB writer: indexed triangles, PBR materials and a real camera.

use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};

use super::lantern::{self, Mesh, Scene};

struct Buffer {
    bytes: Vec<u8>,
    views: Vec<Value>,
    accessors: Vec<Value>,
}

impl Buffer {
    fn new() -> Self {
        Self {
            bytes: Vec::new(),
            views: Vec::new(),
            accessors: Vec::new(),
        }
    }

    fn accessor(
        &mut self,
        bytes: impl IntoIterator<Item = u8>,
        count: usize,
        kind: &str,
        component: u32,
        target: u32,
        bounds: Option<([f32; 3], [f32; 3])>,
    ) -> usize {
        while !self.bytes.len().is_multiple_of(4) {
            self.bytes.push(0);
        }
        let start = self.bytes.len();
        self.bytes.extend(bytes);
        let view = self.views.len();
        self.views.push(json!({ "buffer":0, "byteOffset":start, "byteLength":self.bytes.len()-start, "target":target }));
        let mut accessor =
            json!({ "bufferView":view, "componentType":component, "count":count, "type":kind });
        if let Some((min, max)) = bounds {
            accessor["min"] = json!(min);
            accessor["max"] = json!(max);
        }
        let index = self.accessors.len();
        self.accessors.push(accessor);
        index
    }

    fn mesh(&mut self, mesh: &Mesh, material: usize, name: &str) -> Result<Value> {
        ensure!(
            !mesh.positions.is_empty() && mesh.normals.len() == mesh.positions.len(),
            "invalid mesh {name}"
        );
        ensure!(
            mesh.indices.iter().all(
                |&index| usize::try_from(index).is_ok_and(|index| index < mesh.positions.len())
            ),
            "out of bounds mesh indices {name}"
        );
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        for position in &mesh.positions {
            for axis in 0..3 {
                min[axis] = min[axis].min(position[axis]);
                max[axis] = max[axis].max(position[axis]);
            }
        }
        let positions = self.accessor(
            mesh.positions
                .iter()
                .flatten()
                .flat_map(|x| x.to_le_bytes()),
            mesh.positions.len(),
            "VEC3",
            5126,
            34962,
            Some((min, max)),
        );
        let normals = self.accessor(
            mesh.normals.iter().flatten().flat_map(|x| x.to_le_bytes()),
            mesh.normals.len(),
            "VEC3",
            5126,
            34962,
            None,
        );
        let indices = self.accessor(
            mesh.indices.iter().flat_map(|index| index.to_le_bytes()),
            mesh.indices.len(),
            "SCALAR",
            5125,
            34963,
            None,
        );
        Ok(
            json!({ "name":name, "primitives":[{ "attributes":{"POSITION":positions,"NORMAL":normals}, "indices":indices, "material":material, "mode":4 }] }),
        )
    }
}

pub(super) fn encode(scene: &Scene, meshes: &[Mesh]) -> Result<Vec<u8>> {
    ensure!(
        meshes.len() == scene.nodes.len(),
        "geometry/object count mismatch"
    );
    let mut buffer = Buffer::new();
    let gltf_meshes = meshes
        .iter()
        .zip(&scene.nodes)
        .map(|(mesh, node)| buffer.mesh(mesh, node.material, &node.name))
        .collect::<Result<Vec<_>>>()?;
    let materials = scene.materials.iter().map(|material| {
        ensure!(material.color.iter().all(|c| c.is_finite() && (0.0..=1.0).contains(c)) && (0.0..=1.0).contains(&material.metallic) && (0.0..=1.0).contains(&material.roughness) && material.emission.is_finite()
                && material.emission >= 0.0, "invalid material {}", material.name);
        let [r, g, b] = material.color;
        let mut value = json!({"name":material.name, "doubleSided":true, "pbrMetallicRoughness":{"baseColorFactor":[r,g,b,1.0], "metallicFactor":material.metallic, "roughnessFactor":material.roughness}});
        if material.emission > 0.0 {
            value["emissiveFactor"] = json!(material.color);
            value["extensions"] = json!({"KHR_materials_emissive_strength":{"emissiveStrength":material.emission}});
        }
        Ok(value)
    }).collect::<Result<Vec<_>>>()?;
    let mut nodes: Vec<_> = scene
        .nodes
        .iter()
        .enumerate()
        .map(|(i, node)| json!({"name":node.name, "mesh":i}))
        .collect();
    nodes.push(json!({"name":scene.camera.name,"camera":0,"matrix":lantern::camera_matrix(&scene.camera)?}));
    // glTF has no disk area-light model. Named nodes preserve the exact original
    // settings for authoring, rather than silently substituting point lights.
    for light in &scene.lights {
        let camera = lantern::Camera {
            name: light["name"].as_str().context("light name")?.into(),
            position: serde_json::from_value(light["position"].clone())?,
            target: serde_json::from_value(light["target"].clone())?,
            orthographic_scale: 1.0,
        };
        nodes.push(json!({"name":camera.name,"matrix":lantern::camera_matrix(&camera)?,"extras":{"baylee_area_light":light}}));
    }
    let height = scene.render["height"].as_f64().context("render height")?;
    let width = scene.render["width"].as_f64().context("render width")?;
    ensure!(
        height > 0.0 && width > 0.0 && scene.camera.orthographic_scale > 0.0,
        "invalid portrait dimensions"
    );
    let ymag = scene.camera.orthographic_scale / 2.0;
    let document = json!({
        "asset":{"version":"2.0","generator":"baylee-tools art lantern (native Rust)"},
        "extensionsUsed":["KHR_materials_emissive_strength"],
        "scene":0,
        "scenes":[{"name":scene.name,"nodes":(0..nodes.len()).collect::<Vec<_>>()}],
        "nodes":nodes,"meshes":gltf_meshes,"materials":materials,
        "cameras":[{"name":scene.camera.name,"type":"orthographic","orthographic":{"xmag":ymag*width/height,"ymag":ymag,"znear":0.1,"zfar":1000.0}}],
        "buffers":[{"byteLength":buffer.bytes.len()}],"bufferViews":buffer.views,"accessors":buffer.accessors,
        "extras":{"baylee":{"schema":1,"world":scene.world,"render":scene.render,"lights":scene.lights,
            "boundaries":["Disk area lights, world shader, Cycles and AgX are authoring metadata; glTF cannot execute them.","Native analytic bevel/tube meshes approximate Blender bevel modifiers, weighted normals and POLY bevel tessellation."]}}
    });
    let mut json = serde_json::to_vec(&document)?;
    while !json.len().is_multiple_of(4) {
        json.push(b' ');
    }
    while !buffer.bytes.len().is_multiple_of(4) {
        buffer.bytes.push(0);
    }
    let length = 12 + 8 + json.len() + 8 + buffer.bytes.len();
    let length = u32::try_from(length).context("GLB exceeds its 32-bit chunk size")?;
    let mut result = Vec::with_capacity(length as usize);
    for value in [
        0x4654_6C67,
        2,
        length,
        u32::try_from(json.len())?,
        0x4E4F_534A,
    ] {
        result.extend(value.to_le_bytes());
    }
    result.extend(json);
    result.extend(u32::try_from(buffer.bytes.len())?.to_le_bytes());
    result.extend(0x004E_4942_u32.to_le_bytes());
    result.extend(buffer.bytes);
    Ok(result)
}

#[cfg(test)]
// Exact equality is intentional for seam samples and authored JSON constants.
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn document(bytes: &[u8]) -> Value {
        assert_eq!(&bytes[..4], b"glTF");
        let length = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
        serde_json::from_slice(&bytes[20..20 + length]).unwrap()
    }

    #[test]
    fn native_glb_has_aligned_in_bounds_buffers_and_authored_scene_metadata() {
        let scene: Scene =
            serde_json::from_str(include_str!("../../../../art/baylee/lantern.json")).unwrap();
        let bytes = encode(&scene, &lantern::meshes(&scene).unwrap()).unwrap();
        let doc = document(&bytes);
        assert_eq!(
            u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize,
            bytes.len()
        );
        assert_eq!(doc["meshes"].as_array().unwrap().len(), 50);
        assert_eq!(doc["nodes"].as_array().unwrap().len(), 54);
        assert_eq!(doc["cameras"][0]["orthographic"]["ymag"], 1.04);
        assert_eq!(doc["extras"]["baylee"]["render"]["samples"], 32);
        assert_eq!(doc["extras"]["baylee"]["lights"][1]["power_watts"], 650);
        let buffer_len = doc["buffers"][0]["byteLength"].as_u64().unwrap();
        for view in doc["bufferViews"].as_array().unwrap() {
            let offset = view["byteOffset"].as_u64().unwrap();
            assert_eq!(offset % 4, 0);
            assert!(offset + view["byteLength"].as_u64().unwrap() <= buffer_len);
        }
    }

    #[test]
    fn declarative_scene_keeps_original_export_names_and_pbr_parameters() {
        let original = document(include_bytes!(
            "../../../../art/baylee/wayfinder-lantern.glb"
        ));
        let scene: Scene =
            serde_json::from_str(include_str!("../../../../art/baylee/lantern.json")).unwrap();
        let native = document(&encode(&scene, &lantern::meshes(&scene).unwrap()).unwrap());
        let original_meshes = original["meshes"].as_array().unwrap();
        assert_eq!(original_meshes.len(), 50);
        for node in &scene.nodes {
            assert!(
                original["nodes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|mesh| mesh["name"] == node.name),
                "missing original object {}",
                node.name
            );
        }
        for material in native["materials"].as_array().unwrap() {
            let original = original["materials"]
                .as_array()
                .unwrap()
                .iter()
                .find(|original| original["name"] == material["name"])
                .unwrap();
            // Blender serializes material factors as float32; compare numerically.
            for key in ["metallicFactor", "roughnessFactor"] {
                assert!(
                    (original["pbrMetallicRoughness"][key].as_f64().unwrap()
                        - material["pbrMetallicRoughness"][key].as_f64().unwrap())
                    .abs()
                        < 1e-6
                );
            }
        }
    }
}
