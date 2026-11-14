//! All Minecraft items and blocks (ids + English names), for the item search and
//! the menu editor. The list is generated from the open minecraft-data project.

use egui::{Color32, Painter, Rect, Stroke, StrokeKind, pos2, vec2};
use std::sync::OnceLock;

const RAW: &str = include_str!("../assets/items.txt");

#[derive(Clone, Copy, Debug)]
pub struct ItemInfo {
    /// Id without namespace, e.g. `diamond_sword`.
    pub id: &'static str,
    pub name: &'static str,
    pub stack: u8,
    pub block: bool,
}

pub fn all() -> &'static [ItemInfo] {
    static ALL: OnceLock<Vec<ItemInfo>> = OnceLock::new();
    ALL.get_or_init(|| {
        RAW.lines()
            .filter_map(|l| {
                let mut p = l.split('|');
                Some(ItemInfo {
                    id: p.next()?,
                    name: p.next()?,
                    stack: p.next()?.parse().ok()?,
                    block: p.next()? == "b",
                })
            })
            .collect()
    })
}

pub fn strip(id: &str) -> &str {
    id.trim().trim_start_matches("minecraft:")
}

pub fn get(id: &str) -> Option<&'static ItemInfo> {
    let id = strip(id);
    all().iter().find(|i| i.id == id)
}

/// Display name for an id (falls back to a prettified id).
pub fn display_name(id: &str) -> String {
    match get(id) {
        Some(i) => i.name.to_string(),
        None => {
            let raw = strip(id).rsplit(':').next().unwrap_or("").replace('_', " ");
            let mut c = raw.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Filter {
    All,
    Items,
    Blocks,
}

/// Search by id or name; every word of the query has to match.
pub fn search(query: &str, filter: Filter) -> Vec<&'static ItemInfo> {
    let q = query.trim().to_lowercase().replace("minecraft:", "");
    let words: Vec<&str> = q.split([' ', '_']).filter(|w| !w.is_empty()).collect();
    let mut out: Vec<(u8, &ItemInfo)> = all()
        .iter()
        .filter(|i| match filter {
            Filter::All => true,
            Filter::Items => !i.block,
            Filter::Blocks => i.block,
        })
        .filter_map(|i| {
            if words.is_empty() {
                return Some((2, i));
            }
            let name = i.name.to_lowercase();
            if !words.iter().all(|w| i.id.contains(w) || name.contains(w)) {
                return None;
            }
            let joined = words.join("_");
            let score = if i.id == joined {
                0
            } else if i.id.starts_with(&joined) || name.starts_with(&words.join(" ")) {
                1
            } else {
                2
            };
            Some((score, i))
        })
        .collect();
    out.sort_by_key(|(s, _)| *s);
    out.into_iter().map(|(_, i)| i).collect()
}

/// A fitting color for an item without texture, guessed from its name.
pub fn color_of(id: &str) -> Color32 {
    let id = strip(id);
    const KEYS: &[(&str, (u8, u8, u8))] = &[
        ("diamond", (0x4A, 0xED, 0xD9)),
        ("emerald", (0x17, 0xDD, 0x62)),
        ("netherite", (0x4A, 0x45, 0x4A)),
        ("gold", (0xFD, 0xD8, 0x35)),
        ("iron", (0xD8, 0xD8, 0xD8)),
        ("copper", (0xE0, 0x73, 0x4D)),
        ("redstone", (0xE5, 0x1E, 0x1E)),
        ("lapis", (0x1F, 0x4F, 0xC2)),
        ("amethyst", (0x9A, 0x5C, 0xC6)),
        ("quartz", (0xEC, 0xE6, 0xDF)),
        ("coal", (0x2E, 0x2E, 0x2E)),
        ("obsidian", (0x1B, 0x10, 0x2C)),
        ("lava", (0xF0, 0x6A, 0x10)),
        ("water", (0x3F, 0x76, 0xE4)),
        ("ice", (0x9C, 0xC4, 0xFD)),
        ("snow", (0xF4, 0xFB, 0xFB)),
        ("light_blue", (0x3A, 0xB3, 0xDA)),
        ("light_gray", (0x9D, 0x9D, 0x97)),
        ("white", (0xF0, 0xF0, 0xF0)),
        ("orange", (0xF9, 0x80, 0x1D)),
        ("magenta", (0xC7, 0x4E, 0xBD)),
        ("yellow", (0xFE, 0xD8, 0x3D)),
        ("lime", (0x80, 0xC7, 0x1F)),
        ("pink", (0xF3, 0x8B, 0xAA)),
        ("gray", (0x47, 0x4F, 0x52)),
        ("cyan", (0x16, 0x9C, 0x9C)),
        ("purple", (0x89, 0x32, 0xB8)),
        ("blue", (0x3C, 0x44, 0xAA)),
        ("brown", (0x83, 0x54, 0x32)),
        ("green", (0x5E, 0x7C, 0x16)),
        ("red", (0xB0, 0x2E, 0x26)),
        ("black", (0x1D, 0x1D, 0x21)),
        ("crimson", (0x7E, 0x3A, 0x56)),
        ("warped", (0x2B, 0x68, 0x63)),
        ("cherry", (0xE2, 0xA9, 0xB4)),
        ("birch", (0xC8, 0xB7, 0x7A)),
        ("spruce", (0x72, 0x54, 0x30)),
        ("jungle", (0xA0, 0x73, 0x50)),
        ("acacia", (0xA8, 0x5A, 0x32)),
        ("mangrove", (0x75, 0x36, 0x30)),
        ("bamboo", (0xC3, 0xAD, 0x4F)),
        ("oak", (0xA2, 0x82, 0x4E)),
        ("wood", (0x8F, 0x6B, 0x3A)),
        ("plank", (0xA2, 0x82, 0x4E)),
        ("leaves", (0x4B, 0x8B, 0x2A)),
        ("grass", (0x6B, 0xA8, 0x3C)),
        ("sand", (0xDB, 0xCF, 0xA3)),
        ("dirt", (0x86, 0x60, 0x43)),
        ("deepslate", (0x50, 0x50, 0x54)),
        ("stone", (0x7D, 0x7D, 0x7D)),
        ("brick", (0x96, 0x5A, 0x48)),
        ("prismarine", (0x63, 0xAB, 0x9E)),
        ("glass", (0xC0, 0xE8, 0xF0)),
        ("ender", (0x16, 0x5E, 0x54)),
        ("apple", (0xDD, 0x25, 0x25)),
        ("bread", (0xC9, 0x8E, 0x3F)),
        ("potion", (0xD0, 0x5C, 0xE0)),
        ("book", (0x7A, 0x4E, 0x2A)),
        ("paper", (0xEE, 0xEE, 0xEE)),
        ("stick", (0x8F, 0x6B, 0x3A)),
        ("bone", (0xE8, 0xE5, 0xD2)),
        ("slime", (0x7E, 0xBF, 0x6E)),
        ("honey", (0xF0, 0xA0, 0x24)),
        ("tnt", (0xDB, 0x44, 0x1A)),
        ("glow", (0xF3, 0xD8, 0x8A)),
    ];
    for (k, (r, g, b)) in KEYS {
        if id.contains(k) {
            return Color32::from_rgb(*r, *g, *b);
        }
    }
    // Stable color from the id.
    let h = id.bytes().fold(7u32, |a, b| a.wrapping_mul(31).wrapping_add(b as u32));
    let hue = (h % 360) as f32 / 360.0;
    egui::ecolor::Hsva::new(hue, 0.45, 0.78, 1.0).into()
}

fn shade(c: Color32, f: f32) -> Color32 {
    let m = |x: u8| ((x as f32) * f).clamp(0.0, 255.0) as u8;
    Color32::from_rgb(m(c.r()), m(c.g()), m(c.b()))
}

/// Pixel-style stand-in icon for an item whose texture is not available.
pub fn paint_placeholder(p: &Painter, rect: Rect, id: &str) {
    let id = strip(id);
    let id = id.rsplit(':').next().unwrap_or(id);
    let c = color_of(id);
    let s = rect.width().min(rect.height());
    let r = Rect::from_center_size(rect.center(), vec2(s, s));
    let px = (s / 16.0).max(0.5);
    let is_block = get(id).map(|i| i.block).unwrap_or(false);
    let tool = ["_sword", "_pickaxe", "_axe", "_shovel", "_hoe", "stick", "_rod", "trident", "mace"]
        .iter()
        .any(|t| id.ends_with(t));
    if is_block {
        // Cube: lit top, two shaded sides.
        let top = [
            pos2(r.center().x, r.top() + px),
            pos2(r.right() - px, r.top() + s * 0.25),
            pos2(r.center().x, r.top() + s * 0.5),
            pos2(r.left() + px, r.top() + s * 0.25),
        ];
        let left = [
            pos2(r.left() + px, r.top() + s * 0.25),
            pos2(r.center().x, r.top() + s * 0.5),
            pos2(r.center().x, r.bottom() - px),
            pos2(r.left() + px, r.bottom() - s * 0.25),
        ];
        let right = [
            pos2(r.center().x, r.top() + s * 0.5),
            pos2(r.right() - px, r.top() + s * 0.25),
            pos2(r.right() - px, r.bottom() - s * 0.25),
            pos2(r.center().x, r.bottom() - px),
        ];
        p.add(egui::Shape::convex_polygon(top.to_vec(), shade(c, 1.12), Stroke::NONE));
        p.add(egui::Shape::convex_polygon(left.to_vec(), shade(c, 0.78), Stroke::NONE));
        p.add(egui::Shape::convex_polygon(right.to_vec(), shade(c, 0.58), Stroke::NONE));
    } else if tool {
        // Diagonal tool: handle + head.
        let handle = Color32::from_rgb(0x7A, 0x5A, 0x30);
        p.line_segment(
            [
                pos2(r.left() + s * 0.2, r.bottom() - s * 0.2),
                pos2(r.right() - s * 0.35, r.top() + s * 0.35),
            ],
            Stroke::new(px * 2.2, handle),
        );
        p.line_segment(
            [
                pos2(r.right() - s * 0.5, r.top() + s * 0.5),
                pos2(r.right() - s * 0.16, r.top() + s * 0.16),
            ],
            Stroke::new(px * 3.6, c),
        );
        p.line_segment(
            [
                pos2(r.right() - s * 0.44, r.top() + s * 0.2),
                pos2(r.right() - s * 0.2, r.top() + s * 0.44),
            ],
            Stroke::new(px * 2.0, shade(c, 0.75)),
        );
    } else {
        // Gem-like tile with a highlight.
        let body = r.shrink(s * 0.16);
        p.rect(
            body,
            px * 2.0,
            c,
            Stroke::new(px, shade(c, 0.55)),
            StrokeKind::Inside,
        );
        p.rect_filled(
            Rect::from_min_size(body.min + vec2(px * 2.0, px * 2.0), vec2(s * 0.22, s * 0.12)),
            px,
            shade(c, 1.35),
        );
        p.rect_filled(
            Rect::from_min_max(
                pos2(body.left() + px, body.bottom() - s * 0.16),
                pos2(body.right() - px, body.bottom() - px),
            ),
            px,
            shade(c, 0.75),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_list_is_loaded_and_searchable() {
        assert!(all().len() > 1400);
        assert_eq!(get("minecraft:diamond_sword").unwrap().name, "Diamond Sword");
        assert!(get("stone").unwrap().block);
        let r = search("diamond sw", Filter::All);
        assert_eq!(r[0].id, "diamond_sword");
        assert!(search("oak", Filter::Blocks).iter().all(|i| i.block));
        assert!(search("", Filter::All).len() > 1400);
        assert_eq!(display_name("meinpack:zauber_stab"), "Zauber stab");
    }
}
