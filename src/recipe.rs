//! Crafting recipes of the "Recipe" block: data format, checks and the code /
//! data files for Paper (registered in onEnable) and Fabric (data pack JSON).

use crate::model::Node;

/// Recipe kinds: (id, German, English).
pub const TYPES: &[(&str, crate::i18n::L)] = &[
    ("shaped", ("Werkbank (mit Form)", "Crafting (shaped)")),
    ("shapeless", ("Werkbank (ohne Form)", "Crafting (shapeless)")),
    ("smelting", ("Ofen", "Furnace")),
    ("smoking", ("Räucherofen", "Smoker")),
    ("blasting", ("Schmelzofen", "Blast furnace")),
    ("campfire", ("Lagerfeuer", "Campfire")),
    ("stonecutting", ("Steinsäge", "Stonecutter")),
];

/// Does this kind use only one ingredient (furnaces, stonecutter)?
pub fn single_input(rtype: &str) -> bool {
    !matches!(rtype, "shaped" | "shapeless")
}

/// Does this kind have experience and cooking time?
pub fn cooking(rtype: &str) -> bool {
    matches!(rtype, "smelting" | "smoking" | "blasting" | "campfire")
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Recipe {
    /// 3×3 crafting grid (row by row), item ids, empty = nothing.
    pub grid: [String; 9],
    pub result: String,
    pub amount: u32,
    pub model: String,
    pub name: String,
    /// Lore lines separated by `|`.
    pub lore: String,
}

pub const DEFAULT: &str = "minecraft:diamond\tminecraft:diamond\tminecraft:diamond\t\tminecraft:stick\t\t\tminecraft:stick\t\nminecraft:diamond_pickaxe\t1\t\t&bSuper-Spitzhacke\t";

/// Center slot: where single-ingredient recipes keep their input.
pub const CENTER: usize = 4;

pub fn parse(text: &str) -> Recipe {
    let mut r = Recipe {
        amount: 1,
        ..Default::default()
    };
    let mut lines = text.lines();
    if let Some(g) = lines.next() {
        for (i, c) in g.split('\t').take(9).enumerate() {
            r.grid[i] = c.trim().to_string();
        }
    }
    if let Some(res) = lines.next() {
        let p: Vec<&str> = res.split('\t').collect();
        r.result = p.first().unwrap_or(&"").trim().to_string();
        r.amount = p
            .get(1)
            .and_then(|a| a.trim().parse().ok())
            .unwrap_or(1)
            .clamp(1, 99);
        r.model = p.get(2).unwrap_or(&"").trim().to_string();
        r.name = p.get(3).unwrap_or(&"").trim().to_string();
        r.lore = p.get(4).unwrap_or(&"").trim().to_string();
    }
    r
}

pub fn serialize(r: &Recipe) -> String {
    let clean = |s: &str| s.replace(['\t', '\n', '\r'], " ");
    format!(
        "{}\n{}\t{}\t{}\t{}\t{}",
        r.grid.iter().map(|c| clean(c)).collect::<Vec<_>>().join("\t"),
        clean(&r.result),
        r.amount.clamp(1, 99),
        clean(&r.model),
        clean(&r.name),
        clean(&r.lore)
    )
}

impl Recipe {
    /// Ingredients that are used (for single-input kinds only the first one).
    pub fn inputs(&self, rtype: &str) -> Vec<&str> {
        let all: Vec<&str> = self
            .grid
            .iter()
            .map(|s| s.as_str())
            .filter(|s| !s.is_empty())
            .collect();
        if single_input(rtype) {
            // The center slot first, otherwise the first filled one.
            let c = self.grid[CENTER].as_str();
            if !c.is_empty() {
                return vec![c];
            }
            return all.into_iter().take(1).collect();
        }
        all
    }
}

/// Everything about a recipe block, read from its values.
pub struct Def {
    pub rtype: String,
    pub id: String,
    pub recipe: Recipe,
    pub xp: f64,
    pub seconds: f64,
}

pub fn from_node(node: &Node) -> Def {
    let get = |k: &str, d: &str| node.values.get(k).cloned().unwrap_or_else(|| d.to_string());
    let num = |k: &str, d: f64| {
        get(k, "")
            .replace(',', ".")
            .trim()
            .parse::<f64>()
            .unwrap_or(d)
    };
    let rtype = get("rtype", "shaped");
    let rtype = if TYPES.iter().any(|(k, _)| *k == rtype) {
        rtype
    } else {
        "shaped".into()
    };
    Def {
        id: clean_id(&get("rid", "mein_rezept")),
        recipe: parse(&get("recipe", DEFAULT)),
        xp: num("xp", 0.1).max(0.0),
        seconds: num("seconds", 10.0).max(0.05),
        rtype,
    }
}

/// Recipe ids: lowercase letters, digits, `_`, `-`, `/` and `.`.
pub fn clean_id(s: &str) -> String {
    let id: String = s
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c == ' ' { '_' } else { c })
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '/' | '.'))
        .collect();
    if id.is_empty() { "rezept".into() } else { id }
}

/// What is wrong with the recipe, if anything (German, for the problem list).
pub fn problem(d: &Def) -> Option<String> {
    if d.recipe.result.trim().is_empty() {
        return Some("Das Rezept hat kein Ergebnis".into());
    }
    if d.recipe.inputs(&d.rtype).is_empty() {
        return Some("Das Rezept hat keine Zutaten".into());
    }
    None
}

fn full_id(id: &str) -> String {
    let id = id.trim();
    if id.contains(':') {
        id.to_string()
    } else {
        format!("minecraft:{id}")
    }
}

/// Java string literal.
fn js(s: &str) -> String {
    crate::codegen::java_str(s)
}

/// Paper: one statement for onEnable.
pub fn paper_code(d: &Def) -> String {
    let r = &d.recipe;
    let grid: Vec<String> = if single_input(&d.rtype) {
        d.recipe.inputs(&d.rtype).iter().map(|s| js(&full_id(s))).collect()
    } else {
        r.grid
            .iter()
            .map(|s| if s.is_empty() { js("") } else { js(&full_id(s)) })
            .collect()
    };
    format!(
        "Util.addRecipe({}, {}, new String[] {{ {} }}, Util.customItem({}, {}, {}, {}, {}), {}, {});",
        js(&d.id),
        js(&d.rtype),
        grid.join(", "),
        js(&full_id(&r.result)),
        r.amount,
        js(&r.model),
        js(&r.name),
        js(&r.lore),
        d.xp,
        d.seconds
    )
}

/// `&a` color codes to `§a` (they show in item names from data packs).
fn legacy(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '&'
            && chars
                .peek()
                .map(|n| "0123456789abcdefklmnorABCDEFKLMNOR".contains(*n))
                .unwrap_or(false)
        {
            out.push('§');
        } else {
            out.push(c);
        }
    }
    out
}

/// Fabric: recipe as data pack JSON (data/<modid>/recipe/<id>.json).
pub fn fabric_json(d: &Def) -> String {
    use serde_json::{Map, Value, json};
    let r = &d.recipe;
    let mut result = Map::new();
    result.insert("id".into(), json!(full_id(&r.result)));
    result.insert("count".into(), json!(r.amount.max(1)));
    let mut comps = Map::new();
    if !r.model.trim().is_empty() {
        comps.insert("minecraft:item_model".into(), json!(r.model.trim()));
    }
    if !r.name.trim().is_empty() {
        comps.insert(
            "minecraft:custom_name".into(),
            json!({ "text": legacy(&r.name), "italic": false }),
        );
    }
    let lore: Vec<Value> = r
        .lore
        .split('|')
        .filter(|l| !l.trim().is_empty())
        .map(|l| json!({ "text": legacy(l), "italic": false }))
        .collect();
    if !lore.is_empty() {
        comps.insert("minecraft:lore".into(), Value::Array(lore));
    }
    if !comps.is_empty() {
        result.insert("components".into(), Value::Object(comps));
    }
    let result = Value::Object(result);
    let inputs: Vec<String> = r.inputs(&d.rtype).iter().map(|s| full_id(s)).collect();
    let v = match d.rtype.as_str() {
        "shapeless" => json!({
            "type": "minecraft:crafting_shapeless",
            "category": "misc",
            "ingredients": inputs,
            "result": result
        }),
        "stonecutting" => json!({
            "type": "minecraft:stonecutting",
            "ingredient": inputs.first().cloned().unwrap_or_default(),
            "result": result
        }),
        t if cooking(t) => {
            let (ty, cat) = match t {
                "smoking" => ("minecraft:smoking", "food"),
                "blasting" => ("minecraft:blasting", "misc"),
                "campfire" => ("minecraft:campfire_cooking", "food"),
                _ => ("minecraft:smelting", "misc"),
            };
            json!({
                "type": ty,
                "category": cat,
                "ingredient": inputs.first().cloned().unwrap_or_default(),
                "result": result,
                "experience": d.xp,
                "cookingtime": ((d.seconds * 20.0).round() as i64).max(1)
            })
        }
        _ => {
            let (pattern, key) = shape(&r.grid);
            json!({
                "type": "minecraft:crafting_shaped",
                "category": "misc",
                "pattern": pattern,
                "key": key,
                "result": result
            })
        }
    };
    serde_json::to_string_pretty(&v).unwrap_or_default()
}

/// Shaped pattern trimmed to the used area, with one letter per ingredient.
pub fn shape(grid: &[String; 9]) -> (Vec<String>, serde_json::Map<String, serde_json::Value>) {
    let used: Vec<usize> = (0..9).filter(|i| !grid[*i].is_empty()).collect();
    let mut key = serde_json::Map::new();
    if used.is_empty() {
        return (vec![], key);
    }
    let (r0, r1) = (
        used.iter().map(|i| i / 3).min().unwrap(),
        used.iter().map(|i| i / 3).max().unwrap(),
    );
    let (c0, c1) = (
        used.iter().map(|i| i % 3).min().unwrap(),
        used.iter().map(|i| i % 3).max().unwrap(),
    );
    let mut letters: Vec<(String, char)> = vec![];
    let mut pattern = vec![];
    for row in r0..=r1 {
        let mut line = String::new();
        for col in c0..=c1 {
            let item = &grid[row * 3 + col];
            if item.is_empty() {
                line.push(' ');
                continue;
            }
            let id = full_id(item);
            let ch = match letters.iter().find(|(i, _)| *i == id) {
                Some((_, c)) => *c,
                None => {
                    let c = (b'a' + letters.len() as u8) as char;
                    letters.push((id.clone(), c));
                    key.insert(c.to_string(), serde_json::json!(id));
                    c
                }
            };
            line.push(ch);
        }
        pattern.push(line);
    }
    (pattern, key)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn def(rtype: &str, text: &str) -> Def {
        Def {
            rtype: rtype.into(),
            id: "test".into(),
            recipe: parse(text),
            xp: 0.5,
            seconds: 10.0,
        }
    }

    #[test]
    fn round_trip_and_shapes() {
        let r = parse(DEFAULT);
        assert_eq!(r.grid[0], "minecraft:diamond");
        assert_eq!(r.grid[4], "minecraft:stick");
        assert_eq!(r.result, "minecraft:diamond_pickaxe");
        assert_eq!(parse(&serialize(&r)), r);
        let (p, k) = shape(&r.grid);
        assert_eq!(p, vec!["aaa", " b ", " b "]);
        assert_eq!(k["a"], "minecraft:diamond");
        assert_eq!(k["b"], "minecraft:stick");
        // Trimmed: a 2x2 in the bottom right corner.
        let mut g: [String; 9] = Default::default();
        g[4] = "stone".into();
        g[8] = "stone".into();
        assert_eq!(shape(&g).0, vec!["a ", " a"]);
    }

    #[test]
    fn fabric_recipes_are_valid_json() {
        let d = def("shaped", DEFAULT);
        let v: serde_json::Value = serde_json::from_str(&fabric_json(&d)).unwrap();
        assert_eq!(v["type"], "minecraft:crafting_shaped");
        assert_eq!(v["result"]["id"], "minecraft:diamond_pickaxe");
        assert_eq!(v["result"]["components"]["minecraft:custom_name"]["text"], "§bSuper-Spitzhacke");
        let d = def("smelting", "\t\t\t\tminecraft:raw_iron\t\t\t\t\nminecraft:iron_ingot\t2\t\t\t");
        let v: serde_json::Value = serde_json::from_str(&fabric_json(&d)).unwrap();
        assert_eq!(v["ingredient"], "minecraft:raw_iron");
        assert_eq!(v["cookingtime"], 200);
        assert_eq!(v["result"]["count"], 2);
        assert!(v["result"].get("components").is_none());
        assert!(problem(&def("shaped", "\t\t\t\t\t\t\t\t\nminecraft:stone\t1\t\t\t")).is_some());
        assert!(paper_code(&def("stonecutting", "\t\t\t\tstone\t\t\t\t\nstone_bricks\t1\t\t\t"))
            .contains("new String[] { \"minecraft:stone\" }"));
    }
}
