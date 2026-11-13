//! Resource packs (.zip with textures and models) that get bundled into the
//! generated plugin or mod.
//!
//! * Paper: the zip is put into the plugin jar; the plugin serves it over a tiny
//!   web server and sends it to players when they join.
//! * Fabric: the `assets/` folder is copied into the mod jar, so the textures
//!   load like any mod's own textures.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::RwLock;

/// Item model ids of the current project's pack, used by the model picker.
static MODELS: RwLock<Vec<String>> = RwLock::new(Vec::new());

pub fn set_known_models(m: Vec<String>) {
    if let Ok(mut w) = MODELS.write() {
        *w = m;
    }
}

pub fn known_models() -> Vec<String> {
    MODELS.read().map(|m| m.clone()).unwrap_or_default()
}

#[derive(Clone, Debug, Default)]
pub struct PackInfo {
    pub description: String,
    pub has_mcmeta: bool,
    /// Ids usable as item model (e.g. `meinpack:zauberstab`).
    pub item_models: Vec<String>,
    /// Item model definitions that are generated automatically on export.
    pub generated: Vec<String>,
    /// Zip paths of textures (relative to the pack root).
    pub textures: Vec<String>,
    pub namespaces: Vec<String>,
    #[allow(dead_code)]
    pub files: usize,
    pub bytes: u64,
}

/// All files of the pack, relative to the pack root (the folder with pack.mcmeta / assets).
fn read_entries(path: &Path) -> Result<BTreeMap<String, Vec<u8>>, String> {
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
        let mut buf = Vec::with_capacity(f.size() as usize);
        f.read_to_end(&mut buf).map_err(|e| e.to_string())?;
        out.insert(rel.to_string(), buf);
    }
    Ok(out)
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
fn missing_item_defs(entries: &BTreeMap<String, Vec<u8>>) -> Vec<(String, String)> {
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

pub fn inspect(path: &Path) -> Result<PackInfo, String> {
    let entries = read_entries(path)?;
    let mut info = PackInfo {
        files: entries.len(),
        bytes: entries.values().map(|v| v.len() as u64).sum(),
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
    let mut ns = BTreeSet::new();
    for name in entries.keys() {
        if let Some(rest) = name.strip_prefix("assets/") {
            if let Some((n, _)) = rest.split_once('/') {
                ns.insert(n.to_string());
            }
        }
        if let Some((n, p)) = split_asset(name, "items") {
            info.item_models.push(format!("{n}:{p}"));
        }
        if name.ends_with(".png") && name.contains("/textures/") {
            info.textures.push(name.clone());
        }
    }
    for (n, p) in missing_item_defs(&entries) {
        let id = format!("{n}:{p}");
        info.generated.push(id.clone());
        info.item_models.push(id);
    }
    info.item_models.sort();
    info.item_models.dedup();
    info.namespaces = ns.into_iter().collect();
    Ok(info)
}

/// Pack files ready for export, with generated item model definitions and pack.mcmeta.
pub fn export_entries(path: &Path, description: &str) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let mut entries = read_entries(path)?;
    for (ns, p) in missing_item_defs(&entries) {
        let json = format!(
            "{{\n  \"model\": {{\n    \"type\": \"minecraft:model\",\n    \"model\": \"{ns}:item/{p}\"\n  }}\n}}\n"
        );
        entries.insert(format!("assets/{ns}/items/{p}.json"), json.into_bytes());
    }
    if !entries.contains_key("pack.mcmeta") {
        let desc = serde_json::to_string(description).unwrap_or_else(|_| "\"\"".into());
        let meta = format!(
            "{{\n  \"pack\": {{\n    \"description\": {desc},\n    \"pack_format\": 75,\n    \"supported_formats\": [0, 999],\n    \"min_format\": 0,\n    \"max_format\": 999\n  }}\n}}\n"
        );
        entries.insert("pack.mcmeta".into(), meta.into_bytes());
    }
    Ok(entries)
}

/// The pack as a fresh .zip (for Paper).
pub fn export_zip(path: &Path, description: &str) -> Result<Vec<u8>, String> {
    let entries = export_entries(path, description)?;
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut zw = zip::ZipWriter::new(&mut buf);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for (name, data) in &entries {
            zw.start_file(name.as_str(), opts)
                .map_err(|e| e.to_string())?;
            zw.write_all(data).map_err(|e| e.to_string())?;
        }
        zw.finish().map_err(|e| e.to_string())?;
    }
    Ok(buf.into_inner())
}

/// Small preview image of a texture inside the pack.
pub fn texture_preview(path: &Path, entry: &str) -> Option<egui::ColorImage> {
    let file = std::fs::File::open(path).ok()?;
    let mut zip = zip::ZipArchive::new(file).ok()?;
    let mut data = vec![];
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).ok()?;
        if f.name().replace('\\', "/").ends_with(entry) {
            f.read_to_end(&mut data).ok()?;
            break;
        }
    }
    let img = image::load_from_memory(&data).ok()?.to_rgba8();
    // Animated textures are vertical strips: show the first frame.
    let w = img.width();
    let h = img.height().min(w.max(1));
    let frame = image::imageops::crop_imm(&img, 0, 0, w, h).to_image();
    Some(egui::ColorImage::from_rgba_unmultiplied(
        [frame.width() as usize, frame.height() as usize],
        frame.as_raw(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_zip(files: &[(&str, &str)]) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("mcb-pack-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("p{}.zip", files.len()));
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
        let p = make_zip(&[
            (
                "MeinPack/pack.mcmeta",
                r#"{"pack":{"pack_format":75,"description":"Test"}}"#,
            ),
            ("MeinPack/assets/magie/models/item/stab.json", "{}"),
            ("MeinPack/assets/magie/items/ring.json", "{}"),
            ("MeinPack/assets/magie/textures/item/stab.png", "x"),
        ]);
        let info = inspect(&p).unwrap();
        assert_eq!(info.description, "Test");
        assert_eq!(info.item_models, vec!["magie:ring", "magie:stab"]);
        assert_eq!(info.generated, vec!["magie:stab"]);
        assert_eq!(info.textures.len(), 1);
        let e = export_entries(&p, "x").unwrap();
        assert!(e.contains_key("assets/magie/items/stab.json"));
        assert!(e.contains_key("pack.mcmeta"));
    }
}
