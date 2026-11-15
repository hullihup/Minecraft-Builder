//! Resource packs (.zip with textures and models) that get bundled into the
//! generated plugin or mod.
//!
//! * Several packs are merged into one (first pack in the list wins conflicts;
//!   language, sound, atlas and font files are merged instead of replaced).
//! * Per-item rules decide whether an item replaces a vanilla item, is only
//!   obtainable through commands/blocks, or is left out.
//! * Paper: the merged zip is put into the plugin jar; the plugin serves it over a
//!   tiny web server and sends it to players when they join.
//! * Fabric: the `assets/` folder is copied into the mod jar.

use crate::model::{ItemMode, ItemRule};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

/// Item model ids of the current project's pack, used by the model picker.
static MODELS: RwLock<Vec<String>> = RwLock::new(Vec::new());

pub fn set_known_models(m: Vec<String>) {
    if let Ok(mut w) = MODELS.write() {
        *w = m;
    }
}

/// Sound ids of the project's pack (from sounds.json), for the sound picker.
static SOUNDS: RwLock<Vec<String>> = RwLock::new(Vec::new());

pub fn set_known_sounds(s: Vec<String>) {
    if let Ok(mut w) = SOUNDS.write() {
        *w = s;
    }
}

pub fn known_sounds() -> Vec<String> {
    SOUNDS.read().map(|m| m.clone()).unwrap_or_default()
}

pub fn known_models() -> Vec<String> {
    MODELS.read().map(|m| m.clone()).unwrap_or_default()
}

pub type Entries = BTreeMap<String, Vec<u8>>;

/// One item of the (merged) pack as shown in the pack settings.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PackItem {
    /// Rule key: `namespace:name` as it is in the pack files (`minecraft:x` for vanilla replacements).
    pub key: String,
    /// Model id usable in blocks after the rules are applied (empty if not usable).
    pub model: String,
    /// The pack replaces the look of a vanilla item.
    pub vanilla: bool,
    /// The item definition was missing and is generated.
    pub generated: bool,
    /// Zip entry of a texture for previews.
    pub texture: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct PackStat {
    pub path: PathBuf,
    pub files: usize,
    pub bytes: u64,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct PackInfo {
    pub description: String,
    pub has_mcmeta: bool,
    pub items: Vec<PackItem>,
    /// Ids usable as item model (e.g. `meinpack:zauberstab`).
    pub item_models: Vec<String>,
    /// Item model definitions that are generated automatically on export.
    pub generated: Vec<String>,
    /// Zip paths of textures (relative to the pack root).
    pub textures: Vec<String>,
    pub namespaces: Vec<String>,
    pub files: usize,
    pub bytes: u64,
    /// Files that exist in more than one pack (the first pack wins).
    pub conflicts: Vec<String>,
    /// Files that were merged instead of replaced (lang, sounds, ...).
    pub merged: Vec<String>,
    pub packs: Vec<PackStat>,
    /// Custom sounds of the pack (`namespace:name` from sounds.json).
    pub sounds: Vec<String>,
    /// Texture entry -> index of the pack it comes from.
    pub source: BTreeMap<String, usize>,
}

impl PackInfo {
    /// Texture entry for an item or model id, if the pack has one.
    pub fn texture_of(&self, id: &str) -> Option<&str> {
        let id = if id.contains(':') {
            id.to_string()
        } else {
            format!("minecraft:{id}")
        };
        self.items
            .iter()
            .find(|i| i.model == id || i.key == id)
            .and_then(|i| i.texture.as_deref())
    }
}

/// All files of the pack, relative to the pack root (the folder with pack.mcmeta / assets).
/// With `full == false` only JSON/mcmeta contents are read (enough for inspecting).
fn read_entries(path: &Path, full: bool) -> Result<(Entries, u64), String> {
    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut zip =
        zip::ZipArchive::new(file).map_err(|e| format!("Keine gültige .zip-Datei: {e}"))?;
    let mut names = vec![];
    for i in 0..zip.len() {
        if let Ok(f) = zip.by_index(i) {
            if !f.is_dir() {
                names.push(f.name().replace('\\', "/"));
            }
        }
    }
    // Packs are often zipped with a top folder: find the real root.
    let root = names
        .iter()
        .filter_map(|n| {
            if n.ends_with("pack.mcmeta") {
                Some(n.trim_end_matches("pack.mcmeta").to_string())
            } else {
                n.find("assets/").map(|i| n[..i].to_string())
            }
        })
        .min_by_key(|r| r.len())
        .ok_or("In der .zip gibt es weder pack.mcmeta noch einen assets-Ordner.")?;
    let mut out = BTreeMap::new();
    let mut bytes = 0u64;
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).map_err(|e| e.to_string())?;
        if f.is_dir() {
            continue;
        }
        let name = f.name().replace('\\', "/");
        let Some(rel) = name.strip_prefix(&root) else {
            continue;
        };
        if rel.is_empty() || rel.starts_with("__MACOSX") || rel.ends_with(".DS_Store") {
            continue;
        }
        bytes += f.size();
        let mut buf = Vec::new();
        if full || rel.ends_with(".json") || rel.ends_with(".mcmeta") {
            buf.reserve(f.size() as usize);
            f.read_to_end(&mut buf).map_err(|e| e.to_string())?;
        }
        out.insert(rel.to_string(), buf);
    }
    Ok((out, bytes))
}

/// Files whose JSON content is merged across packs instead of replaced.
fn is_mergeable(name: &str) -> bool {
    let Some(rest) = name.strip_prefix("assets/") else {
        return false;
    };
    let Some((_, rest)) = rest.split_once('/') else {
        return false;
    };
    rest == "sounds.json"
        || (rest.starts_with("lang/") && rest.ends_with(".json"))
        || (rest.starts_with("atlases/") && rest.ends_with(".json"))
        || (rest.starts_with("font/") && rest.ends_with(".json"))
}

/// Merges `low` into `high` (values of `high` win; arrays named sources/providers are joined).
fn merge_json(high: &[u8], low: &[u8]) -> Option<Vec<u8>> {
    use serde_json::Value;
    let mut h: Value = serde_json::from_slice(high).ok()?;
    let l: Value = serde_json::from_slice(low).ok()?;
    let (Value::Object(hm), Value::Object(lm)) = (&mut h, l) else {
        return None;
    };
    for (k, v) in lm {
        match hm.get_mut(&k) {
            None => {
                hm.insert(k, v);
            }
            Some(Value::Array(ha)) if k == "sources" || k == "providers" => {
                if let Value::Array(la) = v {
                    for x in la {
                        if !ha.contains(&x) {
                            ha.push(x);
                        }
                    }
                }
            }
            Some(_) => {}
        }
    }
    serde_json::to_vec_pretty(&h).ok()
}

pub struct Merged {
    pub entries: Entries,
    pub conflicts: Vec<String>,
    pub merged: Vec<String>,
    pub packs: Vec<PackStat>,
    pub source: BTreeMap<String, usize>,
    pub bytes: u64,
}

/// Reads all packs and merges them. `paths[0]` has the highest priority.
pub fn load_merged(paths: &[PathBuf], full: bool) -> Result<Merged, String> {
    let mut m = Merged {
        entries: BTreeMap::new(),
        conflicts: vec![],
        merged: vec![],
        packs: vec![],
        source: BTreeMap::new(),
        bytes: 0,
    };
    let mut ok = 0;
    let mut first_err = None;
    for (idx, p) in paths.iter().enumerate() {
        match read_entries(p, full) {
            Err(e) => {
                first_err.get_or_insert(e.clone());
                m.packs.push(PackStat {
                    path: p.clone(),
                    error: Some(e),
                    ..Default::default()
                });
            }
            Ok((entries, bytes)) => {
                ok += 1;
                m.packs.push(PackStat {
                    path: p.clone(),
                    files: entries.len(),
                    bytes,
                    error: None,
                });
                m.bytes += bytes;
                for (name, data) in entries {
                    match m.entries.get_mut(&name) {
                        None => {
                            m.source.insert(name.clone(), idx);
                            m.entries.insert(name, data);
                        }
                        Some(existing) => {
                            if name == "pack.mcmeta" || name == "pack.png" {
                                continue;
                            }
                            if is_mergeable(&name) {
                                if let Some(j) = merge_json(existing, &data) {
                                    *existing = j;
                                    if !m.merged.contains(&name) {
                                        m.merged.push(name);
                                    }
                                    continue;
                                }
                            }
                            if !m.conflicts.contains(&name) {
                                m.conflicts.push(name);
                            }
                        }
                    }
                }
            }
        }
    }
    if ok == 0 {
        return Err(first_err.unwrap_or_else(|| "Kein Resourcepack".into()));
    }
    Ok(m)
}

/// `assets/<ns>/<kind>/<path>.json` -> (ns, path)
fn split_asset<'a>(name: &'a str, kind: &str) -> Option<(&'a str, &'a str)> {
    let rest = name.strip_prefix("assets/")?;
    let (ns, rest) = rest.split_once('/')?;
    let path = rest
        .strip_prefix(kind)?
        .strip_prefix('/')?
        .strip_suffix(".json")?;
    Some((ns, path))
}

/// Item model definitions that are missing for `models/item/*.json` files.
fn missing_item_defs(entries: &Entries) -> Vec<(String, String)> {
    let have: BTreeSet<(String, String)> = entries
        .keys()
        .filter_map(|n| split_asset(n, "items").map(|(a, b)| (a.to_string(), b.to_string())))
        .collect();
    entries
        .keys()
        .filter_map(|n| split_asset(n, "models/item"))
        .filter(|(ns, p)| !have.contains(&(ns.to_string(), p.to_string())))
        .map(|(ns, p)| (ns.to_string(), p.to_string()))
        .collect()
}

fn item_def_json(model: &str) -> Vec<u8> {
    format!(
        "{{\n  \"model\": {{\n    \"type\": \"minecraft:model\",\n    \"model\": \"{model}\"\n  }}\n}}\n"
    )
    .into_bytes()
}

fn split_id(id: &str) -> (&str, &str) {
    id.split_once(':').unwrap_or(("minecraft", id))
}

/// `ns:path` -> (ns, path) with the default namespace.
fn res_loc(s: &str) -> (String, String) {
    let (a, b) = split_id(s);
    (a.to_string(), b.to_string())
}

fn first_model_ref(v: &serde_json::Value) -> Option<String> {
    match v {
        serde_json::Value::Object(m) => {
            if let Some(serde_json::Value::String(s)) = m.get("model") {
                return Some(s.clone());
            }
            m.values().find_map(first_model_ref)
        }
        serde_json::Value::Array(a) => a.iter().find_map(first_model_ref),
        _ => None,
    }
}

/// Finds a texture (zip entry) that shows what the item `ns:name` looks like.
pub fn find_texture(entries: &Entries, id: &str) -> Option<String> {
    let (ns, name) = split_id(id);
    let has = |p: &str| entries.contains_key(p);
    let mut model = None;
    if let Some(def) = entries.get(&format!("assets/{ns}/items/{name}.json")) {
        if let Ok(v) = serde_json::from_slice::<serde_json::Value>(def) {
            model = first_model_ref(&v);
        }
    }
    let model = model.unwrap_or_else(|| format!("{ns}:item/{name}"));
    let (mns, mpath) = res_loc(&model);
    if let Some(data) = entries.get(&format!("assets/{mns}/models/{mpath}.json")) {
        if let Ok(v) = serde_json::from_slice::<serde_json::Value>(data) {
            if let Some(tex) = v["textures"].as_object() {
                let pick = tex
                    .get("layer0")
                    .or_else(|| tex.get("all"))
                    .or_else(|| tex.get("front"))
                    .or_else(|| tex.get("side"))
                    .or_else(|| tex.values().find(|x| x.as_str().map(|s| !s.starts_with('#')).unwrap_or(false)));
                if let Some(t) = pick.and_then(|x| x.as_str()) {
                    let (tns, tpath) = res_loc(t);
                    let p = format!("assets/{tns}/textures/{tpath}.png");
                    if has(&p) {
                        return Some(p);
                    }
                }
            }
        }
    }
    for kind in ["item", "block"] {
        let p = format!("assets/{ns}/textures/{kind}/{name}.png");
        if has(&p) {
            return Some(p);
        }
    }
    None
}

/// Items of the pack before rules are applied: custom models and vanilla replacements.
fn raw_items(entries: &Entries) -> BTreeMap<String, bool> {
    let mut out: BTreeMap<String, bool> = BTreeMap::new();
    for name in entries.keys() {
        if let Some((ns, p)) = split_asset(name, "items") {
            out.insert(format!("{ns}:{p}"), false);
        }
        if let Some((ns, p)) = split_asset(name, "models/item") {
            out.insert(format!("{ns}:{p}"), false);
        }
        if let Some(rest) = name.strip_prefix("assets/minecraft/textures/item/") {
            if let Some(p) = rest.strip_suffix(".png") {
                if !p.contains('/') {
                    out.insert(format!("minecraft:{p}"), false);
                }
            }
        }
    }
    for (k, v) in out.iter_mut() {
        *v = k.starts_with("minecraft:");
    }
    out
}

fn handheld(name: &str) -> bool {
    ["_sword", "_pickaxe", "_axe", "_shovel", "_hoe", "stick", "_rod", "mace", "trident"]
        .iter()
        .any(|s| name.ends_with(s))
}

/// Valid resource namespace from a project id.
pub fn namespace_for(project_id: &str) -> String {
    let s: String = project_id
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-' || *c == '.')
        .collect();
    if s.is_empty() || s == "minecraft" {
        "custom".into()
    } else {
        s
    }
}

/// Applies the per-item rules. Returns key -> usable model id for the changed items.
fn apply_rules(
    entries: &mut Entries,
    rules: &BTreeMap<String, ItemRule>,
    ns: &str,
) -> BTreeMap<String, String> {
    let mut moved = BTreeMap::new();
    for (key, rule) in rules {
        let (kns, name) = split_id(key);
        let (kns, name) = (kns.to_string(), name.to_string());
        let def_p = format!("assets/{kns}/items/{name}.json");
        let model_p = format!("assets/{kns}/models/item/{name}.json");
        let tex_p = format!("assets/{kns}/textures/item/{name}.png");
        let vanilla = kns == "minecraft";
        match rule.mode {
            ItemMode::Auto => {}
            ItemMode::Off => {
                entries.remove(&def_p);
                entries.remove(&model_p);
                if vanilla {
                    entries.remove(&tex_p);
                    entries.remove(&format!("{tex_p}.mcmeta"));
                }
                moved.insert(key.clone(), String::new());
            }
            ItemMode::CommandOnly if vanilla => {
                // Move the replacement into its own namespace so the vanilla item stays as it is.
                let tex = entries.remove(&tex_p);
                let tex_meta = entries.remove(&format!("{tex_p}.mcmeta"));
                let model = entries.remove(&model_p);
                let def = entries.remove(&def_p);
                if tex.is_none() && model.is_none() && def.is_none() {
                    continue;
                }
                let fix = |data: Vec<u8>| -> Vec<u8> {
                    match String::from_utf8(data) {
                        Ok(s) => s
                            .replace(
                                &format!("\"minecraft:item/{name}\""),
                                &format!("\"{ns}:item/{name}\""),
                            )
                            .replace(&format!("\"item/{name}\""), &format!("\"{ns}:item/{name}\""))
                            .into_bytes(),
                        Err(e) => e.into_bytes(),
                    }
                };
                let has_tex = tex.is_some();
                if let Some(t) = tex {
                    entries.insert(format!("assets/{ns}/textures/item/{name}.png"), t);
                }
                if let Some(t) = tex_meta {
                    entries.insert(format!("assets/{ns}/textures/item/{name}.png.mcmeta"), t);
                }
                let mut has_model = false;
                if let Some(mdl) = model {
                    has_model = true;
                    entries.insert(
                        format!("assets/{ns}/models/item/{name}.json"),
                        if has_tex { fix(mdl) } else { mdl },
                    );
                } else if has_tex {
                    has_model = true;
                    let parent = if handheld(&name) { "handheld" } else { "generated" };
                    entries.insert(
                        format!("assets/{ns}/models/item/{name}.json"),
                        format!("{{\n  \"parent\": \"minecraft:item/{parent}\",\n  \"textures\": {{ \"layer0\": \"{ns}:item/{name}\" }}\n}}\n").into_bytes(),
                    );
                }
                let def_out = match def {
                    Some(d) if has_model => fix(d),
                    Some(d) => d,
                    None => item_def_json(&format!("{ns}:item/{name}")),
                };
                entries.insert(format!("assets/{ns}/items/{name}.json"), def_out);
                moved.insert(key.clone(), format!("{ns}:{name}"));
            }
            ItemMode::CommandOnly => {}
            ItemMode::Replace if !vanilla => {
                let base = rule.base.trim();
                if base.is_empty() {
                    continue;
                }
                let (_, bname) = split_id(base);
                let def = entries
                    .get(&def_p)
                    .cloned()
                    .unwrap_or_else(|| item_def_json(&format!("{kns}:item/{name}")));
                entries.insert(format!("assets/minecraft/items/{bname}.json"), def);
            }
            ItemMode::Replace => {}
        }
    }
    moved
}

fn mcmeta(description: &str) -> Vec<u8> {
    let desc = serde_json::to_string(description).unwrap_or_else(|_| "\"\"".into());
    format!(
        "{{\n  \"pack\": {{\n    \"description\": {desc},\n    \"pack_format\": 75,\n    \"supported_formats\": [0, 999],\n    \"min_format\": 0,\n    \"max_format\": 999\n  }}\n}}\n"
    )
    .into_bytes()
}

/// Looks at the packs (merged, with rules applied) for the sidebar and pickers.
pub fn inspect(
    paths: &[PathBuf],
    rules: &BTreeMap<String, ItemRule>,
    ns: &str,
) -> Result<PackInfo, String> {
    let m = load_merged(paths, false)?;
    let mut entries = m.entries;
    let mut info = PackInfo {
        files: entries.len(),
        bytes: m.bytes,
        conflicts: m.conflicts,
        merged: m.merged,
        packs: m.packs,
        source: m.source,
        ..Default::default()
    };
    if let Some(meta) = entries.get("pack.mcmeta") {
        info.has_mcmeta = true;
        if let Ok(v) = serde_json::from_slice::<serde_json::Value>(meta) {
            info.description = match &v["pack"]["description"] {
                serde_json::Value::String(s) => s.clone(),
                other if !other.is_null() => other.to_string(),
                _ => String::new(),
            };
        }
    }
    for (name, data) in &entries {
        let Some(rest) = name.strip_prefix("assets/") else { continue };
        let Some((ns, "sounds.json")) = rest.split_once('/') else { continue };
        if let Ok(serde_json::Value::Object(m)) = serde_json::from_slice::<serde_json::Value>(data) {
            info.sounds.extend(m.keys().map(|k| format!("{ns}:{k}")));
        }
    }
    info.sounds.sort();
    let raw = raw_items(&entries);
    let raw_tex: BTreeMap<String, Option<String>> = raw
        .keys()
        .map(|k| (k.clone(), find_texture(&entries, k)))
        .collect();
    let generated_raw: BTreeSet<String> = missing_item_defs(&entries)
        .into_iter()
        .map(|(n, p)| format!("{n}:{p}"))
        .collect();
    let moved = apply_rules(&mut entries, rules, ns);
    for (key, vanilla) in &raw {
        let rule = rules.get(key).cloned().unwrap_or_default();
        let model = match moved.get(key) {
            Some(m) => m.clone(),
            None if *vanilla || rule.mode == ItemMode::Off => String::new(),
            None => key.clone(),
        };
        if !model.is_empty() {
            info.item_models.push(model.clone());
        }
        let generated = generated_raw.contains(key) && !vanilla;
        if generated {
            info.generated.push(key.clone());
        }
        info.items.push(PackItem {
            key: key.clone(),
            model,
            vanilla: *vanilla,
            generated,
            texture: raw_tex.get(key).cloned().flatten(),
        });
    }
    let mut ns_set = BTreeSet::new();
    for name in info.source.keys() {
        if let Some(rest) = name.strip_prefix("assets/") {
            if let Some((n, _)) = rest.split_once('/') {
                ns_set.insert(n.to_string());
            }
        }
        if name.ends_with(".png") && name.contains("/textures/") {
            info.textures.push(name.clone());
        }
    }
    info.item_models.sort();
    info.item_models.dedup();
    info.namespaces = ns_set.into_iter().collect();
    Ok(info)
}

/// Pack files ready for export: merged, rules applied, with generated item definitions and pack.mcmeta.
pub fn export_entries(
    paths: &[PathBuf],
    rules: &BTreeMap<String, ItemRule>,
    ns: &str,
    description: &str,
) -> Result<Entries, String> {
    let mut entries = load_merged(paths, true)?.entries;
    apply_rules(&mut entries, rules, ns);
    for (ns, p) in missing_item_defs(&entries) {
        entries.insert(
            format!("assets/{ns}/items/{p}.json"),
            item_def_json(&format!("{ns}:item/{p}")),
        );
    }
    if !entries.contains_key("pack.mcmeta") || paths.len() > 1 {
        entries.insert("pack.mcmeta".into(), mcmeta(description));
    }
    Ok(entries)
}

/// The pack as a fresh .zip (for Paper).
pub fn zip_entries(entries: &Entries) -> Result<Vec<u8>, String> {
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut zw = zip::ZipWriter::new(&mut buf);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for (name, data) in entries {
            zw.start_file(name.as_str(), opts)
                .map_err(|e| e.to_string())?;
            zw.write_all(data).map_err(|e| e.to_string())?;
        }
        zw.finish().map_err(|e| e.to_string())?;
    }
    Ok(buf.into_inner())
}

/// Custom items for the generated give command: (key, base item, model, name, lore).
pub fn give_items(
    info_models: &[String],
    rules: &BTreeMap<String, ItemRule>,
    ns: &str,
) -> Vec<[String; 5]> {
    let mut out: Vec<[String; 5]> = vec![];
    let mut used = BTreeSet::new();
    for model in info_models {
        let (mns, name) = split_id(model);
        // Rule of a moved vanilla item is stored under minecraft:<name>.
        let rule = rules
            .get(model)
            .or_else(|| {
                if mns == ns {
                    rules.get(&format!("minecraft:{name}"))
                } else {
                    None
                }
            })
            .cloned()
            .unwrap_or_default();
        let mut key: String = name.rsplit('/').next().unwrap_or(name).to_string();
        if !used.insert(key.clone()) {
            key = format!("{mns}_{key}");
            used.insert(key.clone());
        }
        let base = if !rule.base.trim().is_empty() {
            rule.base.trim().to_string()
        } else if mns == ns && rules.contains_key(&format!("minecraft:{name}")) {
            format!("minecraft:{name}")
        } else {
            "minecraft:paper".to_string()
        };
        let pretty = if rule.name.trim().is_empty() {
            let mut s = String::from("&f");
            let mut up = true;
            for c in key.chars() {
                if c == '_' || c == '-' {
                    s.push(' ');
                    up = true;
                } else if up {
                    s.extend(c.to_uppercase());
                    up = false;
                } else {
                    s.push(c);
                }
            }
            s
        } else {
            rule.name.clone()
        };
        out.push([key, base, model.clone(), pretty, rule.lore.clone()]);
    }
    out
}

/// Reads one file out of a zip (first entry whose path ends with `entry`).
fn read_zip_entry(path: &Path, entry: &str) -> Option<Vec<u8>> {
    let file = std::fs::File::open(path).ok()?;
    let mut zip = zip::ZipArchive::new(file).ok()?;
    let mut data = vec![];
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).ok()?;
        if f.name().replace('\\', "/").ends_with(entry) {
            f.read_to_end(&mut data).ok()?;
            return Some(data);
        }
    }
    None
}

fn png_preview(data: &[u8]) -> Option<egui::ColorImage> {
    let img = image::load_from_memory(data).ok()?.to_rgba8();
    // Animated textures are vertical strips: show the first frame.
    let w = img.width();
    let h = img.height().min(w.max(1));
    let frame = image::imageops::crop_imm(&img, 0, 0, w, h).to_image();
    Some(egui::ColorImage::from_rgba_unmultiplied(
        [frame.width() as usize, frame.height() as usize],
        frame.as_raw(),
    ))
}

/// Small preview image of a texture inside the pack.
pub fn texture_preview(path: &Path, entry: &str) -> Option<egui::ColorImage> {
    png_preview(&read_zip_entry(path, entry)?)
}

/// Finds an installed Minecraft client jar (for vanilla item textures in the menu editor).
pub fn find_minecraft_jar() -> Option<PathBuf> {
    let base = if cfg!(windows) {
        std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join(".minecraft"))
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join("Library/Application Support/minecraft"))
    } else {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".minecraft"))
    }?;
    let mut jars: Vec<(std::time::SystemTime, PathBuf)> = std::fs::read_dir(base.join("versions"))
        .ok()?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let jar = e.path().join(format!("{name}.jar"));
            let t = std::fs::metadata(&jar).ok()?.modified().ok()?;
            Some((t, jar))
        })
        .collect();
    jars.sort();
    jars.pop().map(|j| j.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_zip(tag: &str, files: &[(&str, &str)]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mcb-pack-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("{tag}.zip"));
        let f = std::fs::File::create(&path).unwrap();
        let mut zw = zip::ZipWriter::new(f);
        for (n, c) in files {
            zw.start_file(*n, zip::write::SimpleFileOptions::default())
                .unwrap();
            zw.write_all(c.as_bytes()).unwrap();
        }
        zw.finish().unwrap();
        path
    }

    #[test]
    fn finds_models_and_generates_missing_definitions() {
        let p = make_zip(
            "single",
            &[
                (
                    "MeinPack/pack.mcmeta",
                    r#"{"pack":{"pack_format":75,"description":"Test"}}"#,
                ),
                (
                    "MeinPack/assets/magie/models/item/stab.json",
                    r#"{"parent":"minecraft:item/handheld","textures":{"layer0":"magie:item/stab"}}"#,
                ),
                ("MeinPack/assets/magie/items/ring.json", "{}"),
                ("MeinPack/assets/magie/textures/item/stab.png", "x"),
            ],
        );
        let rules = BTreeMap::new();
        let info = inspect(&[p.clone()], &rules, "test").unwrap();
        assert_eq!(info.description, "Test");
        assert_eq!(info.item_models, vec!["magie:ring", "magie:stab"]);
        assert_eq!(info.generated, vec!["magie:stab"]);
        assert_eq!(info.textures.len(), 1);
        assert_eq!(
            info.texture_of("magie:stab"),
            Some("assets/magie/textures/item/stab.png")
        );
        let e = export_entries(&[p], &rules, "test", "x").unwrap();
        assert!(e.contains_key("assets/magie/items/stab.json"));
        assert!(e.contains_key("pack.mcmeta"));
    }

    #[test]
    fn merges_several_packs() {
        let a = make_zip(
            "a",
            &[
                ("pack.mcmeta", r#"{"pack":{"pack_format":75,"description":"A"}}"#),
                ("assets/minecraft/lang/en_us.json", r#"{"a":"from a","both":"a wins"}"#),
                ("assets/a/textures/item/x.png", "A"),
                ("assets/shared/textures/item/s.png", "A"),
            ],
        );
        let b = make_zip(
            "b",
            &[
                ("pack.mcmeta", r#"{"pack":{"pack_format":75,"description":"B"}}"#),
                ("assets/minecraft/lang/en_us.json", r#"{"b":"from b","both":"b loses"}"#),
                ("assets/b/models/item/y.json", "{}"),
                ("assets/shared/textures/item/s.png", "B"),
            ],
        );
        let rules = BTreeMap::new();
        let e = export_entries(&[a.clone(), b.clone()], &rules, "test", "Merged").unwrap();
        assert_eq!(e["assets/shared/textures/item/s.png"], b"A");
        assert!(e.contains_key("assets/a/textures/item/x.png"));
        assert!(e.contains_key("assets/b/items/y.json"));
        let lang: serde_json::Value =
            serde_json::from_slice(&e["assets/minecraft/lang/en_us.json"]).unwrap();
        assert_eq!(lang["a"], "from a");
        assert_eq!(lang["b"], "from b");
        assert_eq!(lang["both"], "a wins");
        let info = inspect(&[a, b], &rules, "test").unwrap();
        assert_eq!(info.conflicts, vec!["assets/shared/textures/item/s.png"]);
        assert_eq!(info.packs.len(), 2);
    }

    #[test]
    fn item_rules_change_the_export() {
        let p = make_zip(
            "rules",
            &[
                ("pack.mcmeta", r#"{"pack":{"pack_format":75,"description":"R"}}"#),
                ("assets/minecraft/textures/item/diamond_sword.png", "S"),
                ("assets/minecraft/textures/item/apple.png", "A"),
                ("assets/magie/models/item/stab.json", "{}"),
            ],
        );
        let mut rules = BTreeMap::new();
        rules.insert(
            "minecraft:diamond_sword".to_string(),
            ItemRule {
                mode: ItemMode::CommandOnly,
                ..Default::default()
            },
        );
        rules.insert(
            "minecraft:apple".to_string(),
            ItemRule {
                mode: ItemMode::Off,
                ..Default::default()
            },
        );
        rules.insert(
            "magie:stab".to_string(),
            ItemRule {
                mode: ItemMode::Replace,
                base: "minecraft:stick".into(),
                ..Default::default()
            },
        );
        let e = export_entries(&[p.clone()], &rules, "mein", "x").unwrap();
        assert!(!e.contains_key("assets/minecraft/textures/item/diamond_sword.png"));
        assert!(e.contains_key("assets/mein/textures/item/diamond_sword.png"));
        assert!(e.contains_key("assets/mein/items/diamond_sword.json"));
        let model = String::from_utf8(e["assets/mein/models/item/diamond_sword.json"].clone()).unwrap();
        assert!(model.contains("item/handheld") && model.contains("mein:item/diamond_sword"));
        assert!(!e.contains_key("assets/minecraft/textures/item/apple.png"));
        assert!(e.contains_key("assets/minecraft/items/stick.json"));
        let info = inspect(&[p], &rules, "mein").unwrap();
        assert_eq!(info.item_models, vec!["magie:stab", "mein:diamond_sword"]);
        let give = give_items(&info.item_models, &rules, "mein");
        assert_eq!(give[1][0], "diamond_sword");
        assert_eq!(give[1][1], "minecraft:diamond_sword");
    }
}
