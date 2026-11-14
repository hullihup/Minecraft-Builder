//! Draws items like Minecraft's inventory does: blocks as 3D cubes (from their
//! block models), other items as flat textures. Textures and models are read from
//! zip files: the project's resource packs first, then a vanilla source the user
//! picked (a vanilla template resource pack or the Minecraft client .jar).
//!
//! Nothing of Minecraft ships with the builder; it only reads files on this computer.

use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Size of the rendered pictures in pixels.
pub const SIZE: usize = 64;

struct ZipSource {
    zip: zip::ZipArchive<File>,
    /// Folder inside the zip that holds `assets/` (packs are often zipped with a top folder).
    root: String,
    names: HashSet<String>,
}

impl ZipSource {
    fn open(path: &Path) -> Option<Self> {
        let zip = zip::ZipArchive::new(File::open(path).ok()?).ok()?;
        let all: Vec<String> = zip.file_names().map(|n| n.replace('\\', "/")).collect();
        let root = all
            .iter()
            .filter_map(|n| n.find("assets/").map(|i| n[..i].to_string()))
            .min_by_key(|r| r.len())?;
        let names = all
            .into_iter()
            .filter_map(|n| n.strip_prefix(&root).map(str::to_string))
            .collect();
        Some(Self { zip, root, names })
    }

    fn read(&mut self, name: &str) -> Option<Vec<u8>> {
        if !self.names.contains(name) {
            return None;
        }
        let full = format!("{}{name}", self.root);
        let mut f = self.zip.by_name(&full).ok()?;
        let mut buf = Vec::with_capacity(f.size() as usize);
        f.read_to_end(&mut buf).ok()?;
        Some(buf)
    }
}

type Tex = Arc<image::RgbaImage>;

pub struct Renderer {
    sources: Vec<ZipSource>,
    paths: Vec<PathBuf>,
    textures: HashMap<String, Option<Tex>>,
    models: HashMap<String, Option<Value>>,
}

/// A resolved model: textures by name, cube elements, whether it is a flat item.
#[derive(Default)]
struct Resolved {
    textures: HashMap<String, String>,
    elements: Option<Vec<Value>>,
    generated: bool,
}

/// `ns:path` -> (ns, path), default namespace `minecraft`.
fn split(id: &str) -> (String, String) {
    match id.trim().split_once(':') {
        Some((a, b)) => (a.to_string(), b.to_string()),
        None => ("minecraft".into(), id.trim().to_string()),
    }
}

fn argb_to_rgb(v: &Value) -> Option<[u8; 3]> {
    let n = v.as_i64()? as u32;
    Some([(n >> 16) as u8, (n >> 8) as u8, n as u8])
}

/// Color of a tint source (grass, foliage, constant, dye, potion …).
fn tint_color(t: &Value) -> Option<[u8; 3]> {
    let ty = t["type"].as_str().unwrap_or("").trim_start_matches("minecraft:");
    match ty {
        "constant" => argb_to_rgb(&t["value"]),
        "grass" => Some([0x7C, 0xBD, 0x6B]),
        "foliage" | "dry_foliage" => Some([0x48, 0xB5, 0x18]),
        _ => argb_to_rgb(&t["default"]),
    }
}

/// Picks the model reference and tints out of an item definition (`items/*.json`).
/// Returns (model, tints, special model).
fn pick_model(node: &Value, depth: usize) -> Option<(String, Vec<Option<[u8; 3]>>, Option<Value>)> {
    if depth > 8 {
        return None;
    }
    let ty = node["type"].as_str().unwrap_or("").trim_start_matches("minecraft:");
    match ty {
        "model" => {
            let tints = node["tints"]
                .as_array()
                .map(|a| a.iter().map(tint_color).collect())
                .unwrap_or_default();
            Some((node["model"].as_str()?.to_string(), tints, None))
        }
        "special" => Some((
            node["base"].as_str()?.to_string(),
            vec![],
            Some(node["model"].clone()),
        )),
        "composite" => node["models"].as_array()?.iter().find_map(|m| pick_model(m, depth + 1)),
        _ => {
            // The inventory look, if the item looks different there (e.g. trident).
            if node["property"].as_str().map(|p| p.ends_with("display_context")) == Some(true) {
                if let Some(a) = node["cases"].as_array() {
                    for c in a {
                        let gui = match &c["when"] {
                            Value::String(w) => w == "gui",
                            Value::Array(w) => w.iter().any(|x| x == "gui"),
                            _ => false,
                        };
                        if gui {
                            if let Some(r) = pick_model(&c["model"], depth + 1) {
                                return Some(r);
                            }
                        }
                    }
                }
            }
            // select / condition / range_dispatch / bundle: use the default branch.
            for key in ["fallback", "on_false", "on_true"] {
                if node[key].is_object() {
                    if let Some(r) = pick_model(&node[key], depth + 1) {
                        return Some(r);
                    }
                }
            }
            for key in ["cases", "entries"] {
                if let Some(a) = node[key].as_array() {
                    if let Some(r) = a.iter().find_map(|c| pick_model(&c["model"], depth + 1)) {
                        return Some(r);
                    }
                }
            }
            None
        }
    }
}

impl Renderer {
    /// Opens the zip files (first one wins when a file exists in several).
    pub fn new(paths: &[PathBuf]) -> Self {
        let mut sources = vec![];
        let mut ok = vec![];
        for p in paths {
            if let Some(s) = ZipSource::open(p) {
                sources.push(s);
                ok.push(p.clone());
            }
        }
        Self {
            sources,
            paths: ok,
            textures: HashMap::new(),
            models: HashMap::new(),
        }
    }

    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }

    /// Vanilla items the sources know (from `assets/minecraft/items/*.json`), with
    /// whether each one is a block. Used to add items newer than the built-in list.
    pub fn vanilla_items(&self) -> Vec<(String, bool)> {
        let mut out: Vec<(String, bool)> = self
            .sources
            .iter()
            .flat_map(|s| s.names.iter())
            .filter_map(|n| {
                n.strip_prefix("assets/minecraft/items/")?
                    .strip_suffix(".json")
                    .filter(|x| !x.contains('/') && *x != "air")
            })
            .map(|id| {
                let block = self.has(&format!("assets/minecraft/models/block/{id}.json"));
                (id.to_string(), block)
            })
            .collect();
        out.sort();
        out.dedup();
        out
    }

    fn read(&mut self, name: &str) -> Option<Vec<u8>> {
        self.sources.iter_mut().find_map(|s| s.read(name))
    }

    fn has(&self, name: &str) -> bool {
        self.sources.iter().any(|s| s.names.contains(name))
    }

    fn texture(&mut self, id: &str) -> Option<Tex> {
        let (ns, path) = split(id);
        let key = format!("assets/{ns}/textures/{path}.png");
        if let Some(t) = self.textures.get(&key) {
            return t.clone();
        }
        let t = self
            .read(&key)
            .and_then(|d| image::load_from_memory(&d).ok())
            .map(|img| {
                let img = img.to_rgba8();
                // Animated textures are vertical strips: keep the first frame.
                let w = img.width();
                if img.height() > w {
                    Arc::new(image::imageops::crop_imm(&img, 0, 0, w, w).to_image())
                } else {
                    Arc::new(img)
                }
            });
        self.textures.insert(key, t.clone());
        t
    }

    fn model(&mut self, id: &str) -> Option<Value> {
        let (ns, path) = split(id);
        let key = format!("assets/{ns}/models/{path}.json");
        if let Some(m) = self.models.get(&key) {
            return m.clone();
        }
        let m = self
            .read(&key)
            .and_then(|d| serde_json::from_slice::<Value>(&d).ok());
        self.models.insert(key, m.clone());
        m
    }

    /// Follows the parent chain of a model.
    fn resolve(&mut self, id: &str) -> Resolved {
        let mut r = Resolved::default();
        let mut cur = id.to_string();
        for _ in 0..24 {
            let name = cur.trim_start_matches("minecraft:");
            if name == "builtin/generated" || name == "item/generated" {
                r.generated = true;
                if name == "builtin/generated" {
                    break;
                }
            }
            if name.starts_with("builtin/") {
                break;
            }
            let Some(m) = self.model(&cur) else { break };
            if let Some(t) = m["textures"].as_object() {
                for (k, v) in t {
                    // Either "ns:path" or {"sprite": "ns:path", ...}.
                    if let Some(v) = v.as_str().or_else(|| v["sprite"].as_str()) {
                        r.textures.entry(k.clone()).or_insert_with(|| v.to_string());
                    }
                }
            }
            if r.elements.is_none() {
                if let Some(e) = m["elements"].as_array() {
                    r.elements = Some(e.clone());
                }
            }
            match m["parent"].as_str() {
                Some(p) => cur = p.to_string(),
                None => break,
            }
        }
        r
    }

    fn texture_ref(&mut self, r: &Resolved, name: &str) -> Option<Tex> {
        let mut cur = name.to_string();
        for _ in 0..10 {
            let key = cur.trim_start_matches('#');
            let v = r.textures.get(key)?.clone();
            if v.starts_with('#') {
                cur = v;
            } else {
                return self.texture(&v);
            }
        }
        None
    }

    /// Renders an item (or custom model id) as RGBA pixels of SIZE×SIZE.
    pub fn render(&mut self, id: &str) -> Option<egui::ColorImage> {
        let (ns, name) = split(id);
        let def_path = format!("assets/{ns}/items/{name}.json");
        let def = self
            .read(&def_path)
            .and_then(|d| serde_json::from_slice::<Value>(&d).ok());
        if let Some(d) = &def {
            if d["model"]["type"].as_str().map(|t| t.ends_with("composite")) == Some(true) {
                if let Some(img) = self.composite(&d["model"]) {
                    return Some(img);
                }
            }
        }
        let picked = def.and_then(|v| pick_model(&v["model"], 0));
        let (model, tints, special): (String, Vec<Option<[u8; 3]>>, Option<Value>) = match picked {
            Some(p) => p,
            None => {
                let item = format!("{ns}:item/{name}");
                if self.has(&format!("assets/{ns}/models/item/{name}.json")) {
                    (item, vec![], None)
                } else if self.has(&format!("assets/{ns}/models/block/{name}.json")) {
                    (format!("{ns}:block/{name}"), vec![], None)
                } else if self.has(&format!("assets/{ns}/textures/item/{name}.png")) {
                    // Only a texture: draw it flat.
                    let t = self.texture(&format!("{ns}:item/{name}"))?;
                    return Some(flat(&[(t, None)]));
                } else {
                    return None;
                }
            }
        };
        if let Some(spec) = &special {
            if let Some(img) = self.special(spec) {
                return Some(img);
            }
        }
        let special = special.is_some();
        let r = self.resolve(&model);
        if let Some(elements) = r.elements.clone().filter(|e| !e.is_empty() && !r.generated) {
            return Some(self.cube(&r, &elements, &tints));
        }
        let mut layers = vec![];
        for i in 0..8 {
            match self.texture_ref(&r, &format!("layer{i}")) {
                Some(t) => layers.push((t, tints.get(i).copied().flatten())),
                None => break,
            }
        }
        if !layers.is_empty() {
            return Some(flat(&layers));
        }
        // Special items (chests, beds, heads …) are drawn by the game itself:
        // show their particle texture, as a block if the base looks like one.
        let p = self.texture_ref(&r, "particle")?;
        if special {
            let cube = serde_json::json!([{
                "from": [1, 0, 1], "to": [15, 14, 15],
                "faces": {
                    "up": {"texture": "#particle"},
                    "north": {"texture": "#particle"},
                    "west": {"texture": "#particle"},
                    "east": {"texture": "#particle"},
                    "south": {"texture": "#particle"}
                }
            }]);
            let elements = cube.as_array().cloned().unwrap_or_default();
            return Some(self.cube(&r, &elements, &[]));
        }
        Some(flat(&[(p, None)]))
    }

    /// Items the game draws with entity models: chests, shulker boxes, heads, banners, shields.
    fn special(&mut self, spec: &Value) -> Option<egui::ColorImage> {
        let ty = spec["type"].as_str()?.trim_start_matches("minecraft:");
        let tex_of = |v: &Value, folder: &str| -> Option<String> {
            let (ns, name) = split(v.as_str()?);
            Some(format!("{ns}:entity/{folder}{name}"))
        };
        match ty {
            "chest" => {
                let tex = tex_of(&spec["texture"], "chest/")?;
                let t = self.texture(&tex)?;
                let (tw, th) = (t.width() as f32, t.height() as f32);
                let els = vec![
                    entity_box([1.0, 0.0, 1.0], [15.0, 10.0, 15.0], (0.0, 19.0), (14.0, 10.0, 14.0), tw, th, true),
                    entity_box([1.0, 9.0, 1.0], [15.0, 14.0, 15.0], (0.0, 0.0), (14.0, 5.0, 14.0), tw, th, true),
                    entity_box([7.0, 7.0, 0.0], [9.0, 11.0, 1.0], (0.0, 0.0), (2.0, 4.0, 1.0), tw, th, true),
                ];
                Some(self.entity(&tex, &els))
            }
            "shulker_box" => {
                let tex = tex_of(&spec["texture"], "shulker/")?;
                let t = self.texture(&tex)?;
                let (tw, th) = (t.width() as f32, t.height() as f32);
                let els = vec![
                    entity_box([0.0, 0.0, 0.0], [16.0, 8.0, 16.0], (0.0, 28.0), (16.0, 8.0, 16.0), tw, th, false),
                    entity_box([0.0, 4.0, 0.0], [16.0, 16.0, 16.0], (0.0, 0.0), (16.0, 12.0, 16.0), tw, th, false),
                ];
                Some(self.entity(&tex, &els))
            }
            "head" | "player_head" => {
                let kind = spec["kind"].as_str().unwrap_or("player");
                let (tex, wide) = match (ty, kind) {
                    ("player_head", _) | (_, "player") => ("minecraft:entity/player/wide/steve", 8.0),
                    (_, "zombie") => ("minecraft:entity/zombie/zombie", 8.0),
                    (_, "creeper") => ("minecraft:entity/creeper/creeper", 8.0),
                    (_, "skeleton") => ("minecraft:entity/skeleton/skeleton", 8.0),
                    (_, "wither_skeleton") => ("minecraft:entity/skeleton/wither_skeleton", 8.0),
                    (_, "piglin") => ("minecraft:entity/piglin/piglin", 10.0),
                    _ => return None,
                };
                let t = self.texture(tex)?;
                let (tw, th) = (t.width() as f32, t.height() as f32);
                // Drawn bigger than in the world, like the inventory does.
                let half = wide * 0.75;
                let mut els = vec![entity_box(
                    [8.0 - half, 2.0, 2.0],
                    [8.0 + half, 14.0, 14.0],
                    (0.0, 0.0),
                    (wide, 8.0, 8.0),
                    tw,
                    th,
                    false,
                )];
                if matches!(kind, "player" | "zombie") || ty == "player_head" {
                    // Hat layer, a bit bigger.
                    els.push(entity_box(
                        [1.4, 1.4, 1.4],
                        [14.6, 14.6, 14.6],
                        (32.0, 0.0),
                        (8.0, 8.0, 8.0),
                        tw,
                        th,
                        false,
                    ));
                }
                Some(self.entity(tex, &els))
            }
            "banner" => {
                let t = self
                    .texture("minecraft:entity/banner/banner_base")
                    .or_else(|| self.texture("minecraft:entity/banner/base"))?;
                let color = dye_color(spec["color"].as_str().unwrap_or("white"));
                let k = t.width() as f32 / 64.0;
                Some(flat_region(&t, (1.0 * k, 1.0 * k, 20.0 * k, 40.0 * k), Some(color)))
            }
            "shield" => {
                let t = self
                    .texture("minecraft:entity/shield/base_nopattern")
                    .or_else(|| self.texture("minecraft:entity/shield_base_nopattern"))
                    .or_else(|| self.texture("minecraft:entity/shield/base"))?;
                let k = t.width() as f32 / 64.0;
                Some(flat_region(&t, (1.0 * k, 1.0 * k, 12.0 * k, 22.0 * k), None))
            }
            _ => None,
        }
    }

    fn entity(&mut self, tex: &str, elements: &[Value]) -> egui::ColorImage {
        let mut r = Resolved::default();
        r.textures.insert("e".into(), tex.to_string());
        self.cube(&r, elements, &[])
    }

    /// 3D view like the inventory: seen from above, front (north) on the left.
    fn cube(&mut self, r: &Resolved, elements: &[Value], tints: &[Option<[u8; 3]>]) -> egui::ColorImage {
        let part = Part {
            textures: r.textures.clone(),
            elements: elements.to_vec(),
            tints: tints.to_vec(),
            offset: [0.0; 3],
        };
        self.cube_parts(std::slice::from_ref(&part))
    }

    /// Several models drawn together (e.g. bed head + foot), each moved by its offset.
    fn cube_parts(&mut self, parts: &[Part]) -> egui::ColorImage {
        let n = SIZE;
        let mut color = vec![[0u8; 4]; n * n];
        let mut depth = vec![f32::NEG_INFINITY; n * n];
        let cam = Camera::new();
        // Fit the full blocks (0..16 on every axis, per part) into the picture.
        let (mut min, mut max) = ([f32::MAX; 2], [f32::MIN; 2]);
        for part in parts {
            let o = part.offset;
            for x in [0.0, 16.0] {
                for y in [0.0, 16.0] {
                    for z in [0.0, 16.0] {
                        let p = cam.project([x + o[0], y + o[1], z + o[2]]);
                        min = [min[0].min(p[0]), min[1].min(p[1])];
                        max = [max[0].max(p[0]), max[1].max(p[1])];
                    }
                }
            }
        }
        let scale = (n as f32 * 0.94) / (max[0] - min[0]).max(max[1] - min[1]);
        let off = [
            n as f32 / 2.0 - (min[0] + max[0]) / 2.0 * scale,
            n as f32 / 2.0 - (min[1] + max[1]) / 2.0 * scale,
        ];
        let to_px = |p: [f32; 2]| [p[0] * scale + off[0], p[1] * scale + off[1]];

        for part in parts {
        let r = &Resolved {
            textures: part.textures.clone(),
            ..Default::default()
        };
        let tints: &[Option<[u8; 3]>] = &part.tints;
        let offset = part.offset;
        for el in &part.elements {
            let (Some(from), Some(to)) = (vec3(&el["from"]), vec3(&el["to"])) else {
                continue;
            };
            let rot = Rotation::parse(&el["rotation"]);
            let shade = el["shade"].as_bool().unwrap_or(true);
            let Some(faces) = el["faces"].as_object() else {
                continue;
            };
            for (dir, face) in faces {
                let Some((o, u, v, default_uv)) = face_geometry(dir, from, to) else {
                    continue;
                };
                // Rotate the face with its element.
                let po0 = rot.apply(o);
                let pu = sub(rot.apply(add(o, u)), po0);
                let pv = sub(rot.apply(add(o, v)), po0);
                let po = add(po0, offset);
                let normal = cross(pv, pu);
                // Only faces turned towards the camera.
                if dot(normal, cam.to_camera) <= 0.0 {
                    continue;
                }
                let Some(tex_name) = face["texture"].as_str() else {
                    continue;
                };
                let Some(tex) = self.texture_ref(r, tex_name) else {
                    continue;
                };
                let uv = face["uv"]
                    .as_array()
                    .filter(|a| a.len() == 4)
                    .map(|a| {
                        [
                            a[0].as_f64().unwrap_or(0.0) as f32,
                            a[1].as_f64().unwrap_or(0.0) as f32,
                            a[2].as_f64().unwrap_or(16.0) as f32,
                            a[3].as_f64().unwrap_or(16.0) as f32,
                        ]
                    })
                    .unwrap_or(default_uv);
                let uv_rot = face["rotation"].as_i64().unwrap_or(0);
                let tint = face["tintindex"]
                    .as_i64()
                    .and_then(|i| tints.get(i as usize).copied().flatten())
                    .or_else(|| {
                        // Blocks without item tint info but with a tint index (e.g. grass): grass color.
                        face["tintindex"].as_i64().map(|_| [0x7C, 0xBD, 0x6B])
                    });
                let light = if !shade {
                    1.0
                } else {
                    face_light(normal)
                };
                // Screen-space parallelogram of the face.
                let so = to_px(cam.project(po));
                let su = sub2(to_px(cam.project(add(po, pu))), so);
                let sv = sub2(to_px(cam.project(add(po, pv))), so);
                let det = su[0] * sv[1] - su[1] * sv[0];
                if det.abs() < 1e-4 {
                    continue;
                }
                let d0 = cam.depth(po);
                let du = cam.depth(add(po, pu)) - d0;
                let dv = cam.depth(add(po, pv)) - d0;
                let xs = [so[0], so[0] + su[0], so[0] + sv[0], so[0] + su[0] + sv[0]];
                let ys = [so[1], so[1] + su[1], so[1] + sv[1], so[1] + su[1] + sv[1]];
                let x0 = xs.iter().cloned().fold(f32::MAX, f32::min).floor().max(0.0) as usize;
                let x1 = (xs.iter().cloned().fold(f32::MIN, f32::max).ceil() as usize).min(n);
                let y0 = ys.iter().cloned().fold(f32::MAX, f32::min).floor().max(0.0) as usize;
                let y1 = (ys.iter().cloned().fold(f32::MIN, f32::max).ceil() as usize).min(n);
                let (tw, th) = (tex.width() as f32, tex.height() as f32);
                for py in y0..y1 {
                    for px in x0..x1 {
                        let q = [px as f32 + 0.5 - so[0], py as f32 + 0.5 - so[1]];
                        let s = (q[0] * sv[1] - q[1] * sv[0]) / det;
                        let t = (su[0] * q[1] - su[1] * q[0]) / det;
                        if !(-0.001..=1.001).contains(&s) || !(-0.001..=1.001).contains(&t) {
                            continue;
                        }
                        let d = d0 + s * du + t * dv;
                        let idx = py * n + px;
                        if d <= depth[idx] {
                            continue;
                        }
                        let (s2, t2) = match uv_rot.rem_euclid(360) {
                            90 => (t, 1.0 - s),
                            180 => (1.0 - s, 1.0 - t),
                            270 => (1.0 - t, s),
                            _ => (s, t),
                        };
                        let u = uv[0] + (uv[2] - uv[0]) * s2.clamp(0.0, 0.9999);
                        let v = uv[1] + (uv[3] - uv[1]) * t2.clamp(0.0, 0.9999);
                        let tx = ((u / 16.0 * tw) as i64).clamp(0, tw as i64 - 1) as u32;
                        let ty = ((v / 16.0 * th) as i64).clamp(0, th as i64 - 1) as u32;
                        let texel = tex.get_pixel(tx, ty).0;
                        if texel[3] < 26 {
                            continue;
                        }
                        let mut c = [texel[0] as f32, texel[1] as f32, texel[2] as f32];
                        if let Some(ti) = tint {
                            for k in 0..3 {
                                c[k] = c[k] * ti[k] as f32 / 255.0;
                            }
                        }
                        let a = texel[3];
                        let px_c = [
                            (c[0] * light) as u8,
                            (c[1] * light) as u8,
                            (c[2] * light) as u8,
                        ];
                        if a == 255 {
                            color[idx] = [px_c[0], px_c[1], px_c[2], 255];
                            depth[idx] = d;
                        } else {
                            // Translucent (glass, ice): blend over what is behind.
                            let old = color[idx];
                            let af = a as f32 / 255.0;
                            let mix = |n: u8, o: u8| (n as f32 * af + o as f32 * (1.0 - af)) as u8;
                            color[idx] = [
                                mix(px_c[0], old[0]),
                                mix(px_c[1], old[1]),
                                mix(px_c[2], old[2]),
                                (a as f32 + old[3] as f32 * (1.0 - af)).min(255.0) as u8,
                            ];
                            depth[idx] = d;
                        }
                    }
                }
            }
        }
        }
        let flat: Vec<u8> = color.into_iter().flatten().collect();
        egui::ColorImage::from_rgba_unmultiplied([n, n], &flat)
    }

    /// "composite" items made of several block models (beds): all parts as one picture.
    fn composite(&mut self, node: &Value) -> Option<egui::ColorImage> {
        let models = node["models"].as_array()?;
        if models.len() < 2 {
            return None;
        }
        let mut parts = vec![];
        for m in models {
            if m["type"].as_str()?.trim_start_matches("minecraft:") != "model" {
                return None;
            }
            let r = self.resolve(m["model"].as_str()?);
            let elements = r.elements.clone().filter(|e| !e.is_empty() && !r.generated)?;
            let tints = m["tints"]
                .as_array()
                .map(|a| a.iter().map(tint_color).collect())
                .unwrap_or_default();
            // Translation is in blocks.
            let t = vec3(&m["transformation"]["translation"]).unwrap_or([0.0; 3]);
            parts.push(Part {
                textures: r.textures,
                elements,
                tints,
                offset: [t[0] * 16.0, t[1] * 16.0, t[2] * 16.0],
            });
        }
        Some(self.cube_parts(&parts))
    }
}

/// One model of a picture made of several models.
struct Part {
    textures: HashMap<String, String>,
    elements: Vec<Value>,
    tints: Vec<Option<[u8; 3]>>,
    offset: V3,
}

/// Flat item: layers on top of each other, scaled up without smoothing.
fn flat(layers: &[(Tex, Option<[u8; 3]>)]) -> egui::ColorImage {
    let n = SIZE;
    let mut out = vec![[0u8; 4]; n * n];
    for (tex, tint) in layers {
        let (w, h) = (tex.width() as usize, tex.height() as usize);
        for y in 0..n {
            for x in 0..n {
                let t = tex.get_pixel((x * w / n) as u32, (y * h / n) as u32).0;
                if t[3] == 0 {
                    continue;
                }
                let mut c = [t[0], t[1], t[2]];
                if let Some(ti) = tint {
                    for k in 0..3 {
                        c[k] = (c[k] as u16 * ti[k] as u16 / 255) as u8;
                    }
                }
                let o = &mut out[y * n + x];
                let a = t[3] as f32 / 255.0;
                for k in 0..3 {
                    o[k] = (c[k] as f32 * a + o[k] as f32 * (1.0 - a)) as u8;
                }
                o[3] = (t[3] as f32 + o[3] as f32 * (1.0 - a)).min(255.0) as u8;
            }
        }
    }
    let flat: Vec<u8> = out.into_iter().flatten().collect();
    egui::ColorImage::from_rgba_unmultiplied([n, n], &flat)
}

fn dye_color(name: &str) -> [u8; 3] {
    let c: u32 = match name.trim_start_matches("minecraft:") {
        "orange" => 0xF9801D,
        "magenta" => 0xC74EBD,
        "light_blue" => 0x3AB3DA,
        "yellow" => 0xFED83D,
        "lime" => 0x80C71F,
        "pink" => 0xF38BAA,
        "gray" => 0x474F52,
        "light_gray" => 0x9D9D97,
        "cyan" => 0x169C9C,
        "purple" => 0x8932B8,
        "blue" => 0x3C44AA,
        "brown" => 0x835432,
        "green" => 0x5E7C16,
        "red" => 0xB02E26,
        "black" => 0x1D1D21,
        _ => 0xF9FFFE,
    };
    [(c >> 16) as u8, (c >> 8) as u8, c as u8]
}

/// Part of a texture (x, y, w, h in pixels), scaled to fit and centered.
fn flat_region(tex: &Tex, r: (f32, f32, f32, f32), tint: Option<[u8; 3]>) -> egui::ColorImage {
    let n = SIZE;
    let mut out = vec![[0u8; 4]; n * n];
    let scale = (n as f32 * 0.9) / r.2.max(r.3);
    let (w, h) = (r.2 * scale, r.3 * scale);
    let (x0, y0) = ((n as f32 - w) / 2.0, (n as f32 - h) / 2.0);
    for y in 0..n {
        for x in 0..n {
            let (fx, fy) = (x as f32 + 0.5 - x0, y as f32 + 0.5 - y0);
            if fx < 0.0 || fy < 0.0 || fx >= w || fy >= h {
                continue;
            }
            let tx = ((r.0 + fx / scale) as u32).min(tex.width() - 1);
            let ty = ((r.1 + fy / scale) as u32).min(tex.height() - 1);
            let t = tex.get_pixel(tx, ty).0;
            let mut c = [t[0], t[1], t[2]];
            if let Some(ti) = tint {
                for k in 0..3 {
                    c[k] = (c[k] as u16 * ti[k] as u16 / 255) as u8;
                }
            }
            out[y * n + x] = [c[0], c[1], c[2], t[3]];
        }
    }
    let flat: Vec<u8> = out.into_iter().flatten().collect();
    egui::ColorImage::from_rgba_unmultiplied([n, n], &flat)
}

/// A box of an entity model as block-model element. `uv` is the texture offset and
/// `size` (w, h, d) the box size in texture pixels, unwrapped like Minecraft does.
/// `flipped`: the model is turned upside down (chests, shulker boxes).
fn entity_box(
    from: V3,
    to: V3,
    uv: (f32, f32),
    size: (f32, f32, f32),
    tw: f32,
    th: f32,
    flipped: bool,
) -> Value {
    let (u, v) = uv;
    let (w, h, d) = size;
    // Region (x, y, w, h) in pixels -> uv in 1/16 units.
    let reg = |x: f32, y: f32, rw: f32, rh: f32, rot180: bool, vflip: bool| -> Value {
        let (mut x1, mut y1, mut x2, mut y2) = (x, y, x + rw, y + rh);
        if rot180 {
            std::mem::swap(&mut x1, &mut x2);
            std::mem::swap(&mut y1, &mut y2);
        } else if vflip {
            std::mem::swap(&mut y1, &mut y2);
        }
        serde_json::json!({
            "texture": "#e",
            "uv": [x1 * 16.0 / tw, y1 * 16.0 / th, x2 * 16.0 / tw, y2 * 16.0 / th]
        })
    };
    let top = (u + d, v, w, d);
    let bottom = (u + d + w, v, w, d);
    let east = (u, v + d, d, h);
    let front = (u + d, v + d, w, h);
    let west = (u + d + w, v + d, d, h);
    let back = (u + d + d + w, v + d, w, h);
    let faces = if flipped {
        serde_json::json!({
            "up": reg(bottom.0, bottom.1, bottom.2, bottom.3, false, true),
            "north": reg(back.0, back.1, back.2, back.3, true, false),
            "west": reg(west.0, west.1, west.2, west.3, true, false),
            "east": reg(east.0, east.1, east.2, east.3, true, false),
            "south": reg(front.0, front.1, front.2, front.3, true, false),
            "down": reg(top.0, top.1, top.2, top.3, false, true)
        })
    } else {
        serde_json::json!({
            "up": reg(top.0, top.1, top.2, top.3, true, false),
            "north": reg(front.0, front.1, front.2, front.3, false, false),
            "west": reg(west.0, west.1, west.2, west.3, false, false),
            "east": reg(east.0, east.1, east.2, east.3, false, false),
            "south": reg(back.0, back.1, back.2, back.3, false, false),
            "down": reg(bottom.0, bottom.1, bottom.2, bottom.3, false, false)
        })
    };
    serde_json::json!({ "from": from, "to": to, "faces": faces })
}

// ------------------------------------------------------------------ geometry

type V3 = [f32; 3];

fn vec3(v: &Value) -> Option<V3> {
    let a = v.as_array()?;
    Some([
        a.first()?.as_f64()? as f32,
        a.get(1)?.as_f64()? as f32,
        a.get(2)?.as_f64()? as f32,
    ])
}
fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn sub2(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] - b[0], a[1] - b[1]]
}
fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Brightness of a face like in the game: top bright, sides darker.
fn face_light(normal: V3) -> f32 {
    let len = dot(normal, normal).sqrt().max(1e-6);
    let n = [normal[0] / len, normal[1] / len, normal[2] / len];
    if n[1] > 0.5 {
        1.0
    } else if n[1] < -0.5 {
        0.5
    } else if n[2].abs() > n[0].abs() {
        0.8
    } else {
        0.6
    }
}

/// Origin (texture corner u=0,v=0), u direction, v direction and default uv of a face.
fn face_geometry(dir: &str, f: V3, t: V3) -> Option<(V3, V3, V3, [f32; 4])> {
    let (dx, dy, dz) = (t[0] - f[0], t[1] - f[1], t[2] - f[2]);
    Some(match dir {
        "up" => ([f[0], t[1], f[2]], [dx, 0.0, 0.0], [0.0, 0.0, dz], [f[0], f[2], t[0], t[2]]),
        "down" => (
            [f[0], f[1], t[2]],
            [dx, 0.0, 0.0],
            [0.0, 0.0, -dz],
            [f[0], 16.0 - t[2], t[0], 16.0 - f[2]],
        ),
        "north" => (
            [t[0], t[1], f[2]],
            [-dx, 0.0, 0.0],
            [0.0, -dy, 0.0],
            [16.0 - t[0], 16.0 - t[1], 16.0 - f[0], 16.0 - f[1]],
        ),
        "south" => (
            [f[0], t[1], t[2]],
            [dx, 0.0, 0.0],
            [0.0, -dy, 0.0],
            [f[0], 16.0 - t[1], t[0], 16.0 - f[1]],
        ),
        "east" => (
            [t[0], t[1], t[2]],
            [0.0, 0.0, -dz],
            [0.0, -dy, 0.0],
            [16.0 - t[2], 16.0 - t[1], 16.0 - f[2], 16.0 - f[1]],
        ),
        "west" => (
            [f[0], t[1], f[2]],
            [0.0, 0.0, dz],
            [0.0, -dy, 0.0],
            [f[2], 16.0 - t[1], t[2], 16.0 - f[1]],
        ),
        _ => return None,
    })
}

struct Rotation {
    origin: V3,
    axis: usize,
    angle: f32,
}

impl Rotation {
    fn parse(v: &Value) -> Self {
        let axis = match v["axis"].as_str() {
            Some("x") => 0,
            Some("z") => 2,
            _ => 1,
        };
        Rotation {
            origin: vec3(&v["origin"]).unwrap_or([8.0, 8.0, 8.0]),
            axis,
            angle: v["angle"].as_f64().unwrap_or(0.0) as f32,
        }
    }
    fn apply(&self, p: V3) -> V3 {
        if self.angle == 0.0 {
            return p;
        }
        let (s, c) = self.angle.to_radians().sin_cos();
        let q = sub(p, self.origin);
        let r = match self.axis {
            0 => [q[0], q[1] * c - q[2] * s, q[1] * s + q[2] * c],
            1 => [q[0] * c + q[2] * s, q[1], -q[0] * s + q[2] * c],
            _ => [q[0] * c - q[1] * s, q[0] * s + q[1] * c, q[2]],
        };
        add(r, self.origin)
    }
}

/// Camera above the north-west corner, 30° down (like the inventory).
struct Camera {
    right: V3,
    up: V3,
    to_camera: V3,
}

impl Camera {
    fn new() -> Self {
        let e = 30f32.to_radians();
        let h = std::f32::consts::FRAC_1_SQRT_2;
        // Looking from the north-west, above (-x, +y, -z): north face left, west face right.
        let forward = [e.cos() * h, -e.sin(), e.cos() * h];
        let right = [-h, 0.0, h];
        let up = cross(right, forward);
        Camera {
            right,
            up,
            to_camera: [-forward[0], -forward[1], -forward[2]],
        }
    }
    fn project(&self, p: V3) -> [f32; 2] {
        [dot(p, self.right), -dot(p, self.up)]
    }
    fn depth(&self, p: V3) -> f32 {
        dot(p, self.to_camera)
    }
}

/// Is this zip usable as source of vanilla textures (template pack or client jar)?
pub fn is_vanilla_source(path: &Path) -> bool {
    ZipSource::open(path)
        .map(|s| {
            s.names.contains("assets/minecraft/models/block/cube_all.json")
                && s.names
                    .iter()
                    .any(|n| n.starts_with("assets/minecraft/textures/block/"))
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn png(color: [u8; 4]) -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(16, 16, image::Rgba(color));
        let mut buf = std::io::Cursor::new(Vec::new());
        img.write_to(&mut buf, image::ImageFormat::Png).unwrap();
        buf.into_inner()
    }

    fn make_zip() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mcb-render-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("vanilla.zip");
        let mut zw = zip::ZipWriter::new(File::create(&path).unwrap());
        let o = zip::write::SimpleFileOptions::default();
        let files: Vec<(&str, Vec<u8>)> = vec![
            ("Pack/assets/minecraft/models/block/block.json", br#"{}"#.to_vec()),
            (
                "Pack/assets/minecraft/models/block/cube.json",
                br##"{"parent":"block/block","elements":[{"from":[0,0,0],"to":[16,16,16],"faces":{
                    "down":{"texture":"#down"},"up":{"texture":"#up"},"north":{"texture":"#north"},
                    "south":{"texture":"#south"},"east":{"texture":"#east"},"west":{"texture":"#west"}}}]}"##
                    .to_vec(),
            ),
            (
                "Pack/assets/minecraft/models/block/cube_all.json",
                br##"{"parent":"block/cube","textures":{"down":"#all","up":"#all","north":"#all","south":"#all","east":"#all","west":"#all"}}"##.to_vec(),
            ),
            (
                "Pack/assets/minecraft/models/block/stone.json",
                br#"{"parent":"minecraft:block/cube_all","textures":{"all":"minecraft:block/stone"}}"#.to_vec(),
            ),
            (
                "Pack/assets/minecraft/items/stone.json",
                br#"{"model":{"type":"minecraft:model","model":"minecraft:block/stone"}}"#.to_vec(),
            ),
            ("Pack/assets/minecraft/models/item/generated.json", br#"{"parent":"builtin/generated"}"#.to_vec()),
            (
                "Pack/assets/minecraft/models/item/apple.json",
                br#"{"parent":"minecraft:item/generated","textures":{"layer0":"minecraft:item/apple"}}"#.to_vec(),
            ),
            (
                "Pack/assets/minecraft/items/apple.json",
                br#"{"model":{"type":"minecraft:select","property":"x","cases":[],"fallback":{"type":"minecraft:model","model":"minecraft:item/apple"}}}"#.to_vec(),
            ),
            ("Pack/assets/minecraft/textures/block/stone.png", png([100, 100, 100, 255])),
            ("Pack/assets/minecraft/textures/item/apple.png", png([200, 20, 20, 255])),
        ];
        for (n, d) in files {
            zw.start_file(n, o).unwrap();
            zw.write_all(&d).unwrap();
        }
        zw.finish().unwrap();
        path
    }

    #[test]
    fn renders_blocks_as_cubes_and_items_flat() {
        let path = make_zip();
        assert!(is_vanilla_source(&path));
        let mut r = Renderer::new(std::slice::from_ref(&path));
        // Stone: three shades of gray (top, left, right), transparent corners.
        let img = r.render("minecraft:stone").unwrap();
        assert_eq!(img.size, [SIZE, SIZE]);
        let px = |x: usize, y: usize| img.pixels[y * SIZE + x];
        assert_eq!(px(0, 0).a(), 0, "corner of a cube picture is empty");
        let top = px(SIZE / 2, SIZE / 5);
        let left = px(SIZE / 4, SIZE * 3 / 5);
        let right = px(SIZE * 3 / 4, SIZE * 3 / 5);
        assert_eq!(top.r(), 100);
        assert!(left.r() < top.r() && right.r() < left.r(), "{top:?} {left:?} {right:?}");
        // Apple: flat, fills the picture.
        let img = r.render("apple").unwrap();
        let c = img.pixels[SIZE * SIZE / 2 + SIZE / 2];
        assert_eq!((c.r(), c.g(), c.a()), (200, 20, 255));
        assert!(r.render("minecraft:does_not_exist").is_none());
    }
}
