
use serde_json::{json, Map, Value};

use crate::canonical_json::canonical_json;
use crate::fnv1a;
use crate::height::ops_jsgen::V3;
use crate::height::planet_frame::{anchor_basis, tangent_local_to_dir, Basis};
use crate::world_name::{world_stem, WORLD_DIR};

const DEFAULT_HASH_VERSION: u32 = 2;
const LEGACY_HASH_VERSION: u32 = 1;
const HASH_VERSIONS: [u32; 2] = [1, 2];
const MINIMAP_EXTENT_RADIUS_FRACTION: f64 = 0.25;
const MINIMAP_MAX_EXTENT_M: f64 = 16384.0;
const DEFAULT_MINIMAP_RES: f64 = 256.0;

#[derive(Clone, Debug, PartialEq)]
pub struct Refusal {
    pub name: &'static str,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParseRefusal {
    pub path: &'static str,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Carve {
    pub dir: V3,
    pub inner_m: f64,
    pub outer_m: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TerrainConfig {
    pub seed: Option<f64>,
    pub radius: Option<f64>,
    pub relief_scale: Option<f64>,
    pub anchor_dir: Value,
    pub center: Value,
    pub enabled: bool,
    pub hash_version: u32,
    pub carves: Vec<Carve>,
    pub carve_inputs: Value,
    pub extent: f64,
    pub res: f64,
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64() != Some(0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn or_else(v: Option<&Value>, fallback: Value) -> Value {
    match v {
        Some(v) if truthy(v) => v.clone(),
        _ => fallback,
    }
}

fn finite(v: Option<&Value>) -> Option<f64> {
    v.and_then(Value::as_f64).filter(|n| n.is_finite())
}

fn json_text(v: &Value) -> String {
    serde_json::to_string(v).unwrap()
}

fn json_number(n: f64) -> Value {
    if n.is_finite() && n.fract() == 0.0 && n.abs() < 1e15 {
        json!(n as i64)
    } else {
        Value::from(n)
    }
}

fn js_number(text: &str) -> f64 {
    let t = text.trim();
    let radix = match t.get(..2) {
        Some("0x" | "0X") => 16,
        Some("0o" | "0O") => 8,
        Some("0b" | "0B") => 2,
        _ => 10,
    };
    if t.is_empty() {
        0.0
    } else if radix == 10 {
        t.parse::<f64>().unwrap_or(f64::NAN)
    } else {
        u64::from_str_radix(&t[2..], radix).map_or(f64::NAN, |n| n as f64)
    }
}

pub fn assert_terrain_hash_version(v: &Value) -> Result<u32, Refusal> {
    let versions = HASH_VERSIONS.map(|h| h.to_string()).join(", ");
    v.as_f64()
        .and_then(|n| HASH_VERSIONS.into_iter().find(|h| f64::from(*h) == n))
        .ok_or_else(|| Refusal {
            name: "RangeError",
            message: format!("terrain hashVersion must be one of {versions}, got {}", json_text(v)),
        })
}

pub fn terrain_hash_version_of(tcfg: &Value) -> Result<u32, Refusal> {
    match tcfg.get("hashVersion").filter(|v| !v.is_null()) {
        Some(v) => assert_terrain_hash_version(v),
        None => Ok(DEFAULT_HASH_VERSION),
    }
}

pub fn parse_terrain_hash_override(raw: Option<&str>) -> Result<Option<u32>, Refusal> {
    match raw {
        None | Some("") => Ok(None),
        Some(text) => assert_terrain_hash_version(&json_number(js_number(text))).map(Some),
    }
}

fn parsed_carve(c: &Value) -> Option<Carve> {
    let dir = match c.get("dir")?.as_array()?.as_slice() {
        [a, b, d] => [finite(Some(a))?, finite(Some(b))?, finite(Some(d))?],
        _ => return None,
    };
    let inner_m = finite(c.get("innerM"))?;
    let outer_m = finite(c.get("outerM"))?;
    (inner_m >= 0.0 && outer_m > inner_m).then_some(Carve { dir, inner_m, outer_m })
}

fn v3_of(v: &Value) -> V3 {
    let read = |i: usize| v.get(i).and_then(Value::as_f64).unwrap_or(f64::NAN);
    [read(0), read(1), read(2)]
}

fn carve_refusal(c: &Value) -> Refusal {
    Refusal {
        name: "RangeError",
        message: format!("terrain carve needs center [x, z], radius >= 0 and falloff > 0, got {}", json_text(c)),
    }
}

fn carve_from_center(c: &Value, radius: f64, basis: &Basis) -> Result<Carve, Refusal> {
    let center = c.get("center").filter(|v| truthy(v)).and_then(Value::as_array);
    let x = finite(center.and_then(|a| a.first()));
    let z = finite(center.and_then(|a| a.get(1)));
    let inner_m = c.get("radius").and_then(Value::as_f64).filter(|r| *r >= 0.0);
    let falloff = c.get("falloff").and_then(Value::as_f64).filter(|f| *f > 0.0);
    match (x, z, inner_m, falloff) {
        (Some(x), Some(z), Some(inner_m), Some(falloff)) => Ok(Carve {
            dir: tangent_local_to_dir(basis, radius, x, z),
            inner_m,
            outer_m: inner_m + falloff,
        }),
        _ => Err(carve_refusal(c)),
    }
}

pub fn terrain_carves_of(tcfg: &Value) -> Result<Vec<Carve>, Refusal> {
    let carves = match tcfg.get("carves") {
        None | Some(Value::Null) => return Ok(Vec::new()),
        Some(carves) => carves,
    };
    if terrain_hash_version_of(tcfg)? == LEGACY_HASH_VERSION {
        return Ok(Vec::new());
    }
    let list = carves.as_array().ok_or_else(|| Refusal {
        name: "TypeError",
        message: format!("terrain carves must be an array, got {}", json_text(carves)),
    })?;
    let raw_radius = tcfg.get("radius");
    let radius = raw_radius
        .and_then(Value::as_f64)
        .filter(|r| *r > 0.0)
        .ok_or_else(|| Refusal {
            name: "RangeError",
            message: format!(
                "terrain carves need a positive terrain radius, got {}",
                raw_radius.map_or_else(|| "undefined".to_string(), json_text)
            ),
        })?;
    let basis = anchor_basis(v3_of(&or_else(tcfg.get("anchorDir"), json!([0, 1, 0]))));
    list.iter()
        .map(|c| match parsed_carve(c) {
            Some(carve) => Ok(carve),
            None => carve_from_center(c, radius, &basis),
        })
        .collect()
}

fn minimap_extent_of(tcfg: &Value) -> f64 {
    finite(tcfg.get("minimapExtent")).unwrap_or_else(|| {
        let radius = tcfg.get("radius").and_then(Value::as_f64).unwrap_or(f64::NAN);
        let scaled = radius * MINIMAP_EXTENT_RADIUS_FRACTION;
        if scaled.is_nan() { scaled } else { scaled.min(MINIMAP_MAX_EXTENT_M) }
    })
}

fn minimap_res_of(tcfg: &Value) -> f64 {
    finite(tcfg.get("minimapRes")).unwrap_or(DEFAULT_MINIMAP_RES)
}

pub fn parse_terrain_config(tcfg: &Value) -> Result<TerrainConfig, ParseRefusal> {
    let refuse = |path: &'static str, reason: &str| ParseRefusal { path, reason: reason.to_string() };
    if !tcfg.is_object() {
        return Err(refuse("", "terrain config must be a plain object"));
    }
    if tcfg.get("seed").is_some_and(|s| finite(Some(s)).is_none()) {
        return Err(refuse("seed", "must be a finite number"));
    }
    if tcfg.get("radius").is_some_and(|r| finite(Some(r)).filter(|r| *r > 0.0).is_none()) {
        return Err(refuse("radius", "must be a positive finite number"));
    }
    let hash_version = terrain_hash_version_of(tcfg).map_err(|e| ParseRefusal { path: "hashVersion", reason: e.message })?;
    let carves = terrain_carves_of(tcfg).map_err(|e| ParseRefusal { path: "carves", reason: e.message })?;
    Ok(TerrainConfig {
        seed: finite(tcfg.get("seed")),
        radius: finite(tcfg.get("radius")),
        relief_scale: finite(tcfg.get("reliefScale")),
        anchor_dir: or_else(tcfg.get("anchorDir"), json!([0, 1, 0])),
        center: or_else(tcfg.get("center"), json!([0, 0])),
        enabled: tcfg.get("enabled") != Some(&Value::Bool(false)),
        hash_version,
        carves,
        carve_inputs: if hash_version == LEGACY_HASH_VERSION {
            json!([])
        } else {
            or_else(tcfg.get("carveInputs"), or_else(tcfg.get("carves"), json!([])))
        },
        extent: minimap_extent_of(tcfg),
        res: minimap_res_of(tcfg),
    })
}

pub fn resolve_terrain_config(world_def: &Value) -> Value {
    let terrain_app = Value::from("terrain");
    let entity = world_def
        .get("entities")
        .and_then(Value::as_array)
        .and_then(|entities| entities.iter().find(|e| e.get("app") == Some(&terrain_app)));
    entity
        .and_then(|e| e.get("config"))
        .filter(|c| truthy(c))
        .or_else(|| world_def.get("terrain").filter(|t| truthy(t)))
        .cloned()
        .unwrap_or(Value::Null)
}

#[derive(Clone, Debug, PartialEq)]
pub struct MinimapDescriptor {
    pub base: String,
    pub center: Value,
    pub extent: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MinimapBakeParams {
    pub radius: Option<f64>,
    pub relief_scale: Option<f64>,
    pub hash_version: u32,
    pub carves: Vec<Carve>,
    pub anchor_dir: Value,
    pub extent: f64,
    pub res: f64,
    pub center: Value,
}

fn to_int32(n: f64) -> i32 {
    if !n.is_finite() {
        return 0;
    }
    n.trunc().rem_euclid(4_294_967_296.0) as u32 as i32
}

fn parsed_terrain_of(tcfg: &Value) -> Result<TerrainConfig, Refusal> {
    parse_terrain_config(tcfg).map_err(|p| {
        let prefix = if p.path.is_empty() { String::new() } else { format!("{}: ", p.path) };
        Refusal { name: "TypeError", message: format!("terrain {prefix}{}", p.reason) }
    })
}

fn minimap_base_name(world_id: &str, c: &TerrainConfig) -> String {
    let hash_tag = if c.hash_version == DEFAULT_HASH_VERSION { String::new() } else { format!(".h{}", c.hash_version) };
    let seed = c.seed.map_or(0, to_int32);
    format!("{world_id}.{seed}{hash_tag}.minimap")
}

pub fn minimap_descriptor(world_id: &Value, tcfg: &Value) -> Result<Option<MinimapDescriptor>, Refusal> {
    let Some(world_id) = world_stem(world_id) else {
        return Ok(None);
    };
    if !truthy(tcfg) || tcfg.get("enabled") == Some(&Value::Bool(false)) || finite(tcfg.get("seed")).is_none() {
        return Ok(None);
    }
    let c = parsed_terrain_of(tcfg)?;
    Ok(Some(MinimapDescriptor {
        base: format!("/{WORLD_DIR}/{}", minimap_base_name(world_id, &c)),
        center: c.center,
        extent: c.extent,
    }))
}

pub fn minimap_bake_params(tcfg: &Value) -> Result<MinimapBakeParams, Refusal> {
    let c = parsed_terrain_of(tcfg)?;
    Ok(MinimapBakeParams {
        radius: c.radius,
        relief_scale: c.relief_scale,
        hash_version: c.hash_version,
        carves: c.carves,
        anchor_dir: c.anchor_dir,
        extent: c.extent,
        res: c.res,
        center: c.center,
    })
}

pub fn terrain_bake_key(tcfg: &Value) -> Result<String, Refusal> {
    let c = parsed_terrain_of(tcfg)?;
    let slice = json!({
        "seed": c.seed.map_or(Value::Null, Value::from),
        "radius": c.radius.map_or(Value::Null, Value::from),
        "reliefScale": c.relief_scale.map_or(Value::Null, Value::from),
        "anchorDir": c.anchor_dir,
        "hashVersion": c.hash_version,
        "carves": c.carve_inputs,
    });
    Ok(format!("{:08x}", fnv1a::step_string(fnv1a::FNV1A_32_OFFSET_BASIS, &canonical_json(&slice))))
}

pub fn with_terrain_seed(world_def: &Value, seed: &Value) -> Result<Value, Refusal> {
    let seed = seed.as_f64().filter(|n| n.is_finite() && n.fract() == 0.0).ok_or_else(|| Refusal {
        name: "TypeError",
        message: format!("withTerrainSeed: seed must be an integer, got {}", canonical_json(seed)),
    })?;
    Ok(map_terrain_configs(world_def, &|cfg| reseed_terrain_config(cfg, seed)))
}

fn reseed_terrain_config(cfg: &Map<String, Value>, seed: f64) -> Value {
    let mut reseeded: Map<String, Value> = cfg
        .iter()
        .filter(|(key, _)| key.as_str() != "bakedHeightfield")
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    if cfg.get("seed").and_then(Value::as_f64) != Some(seed) {
        reseeded.insert("seed".to_string(), Value::from(seed));
    }
    if let Some(Value::Object(vegetation)) = cfg.get("vegetation") {
        let mut vegetation = vegetation.clone();
        vegetation.insert("seed".to_string(), Value::from(seed));
        reseeded.insert("vegetation".to_string(), Value::Object(vegetation));
    }
    Value::Object(reseeded)
}

fn map_terrain_configs(world_def: &Value, patch: &dyn Fn(&Map<String, Value>) -> Value) -> Value {
    let Value::Object(def) = world_def else {
        return world_def.clone();
    };
    let mut next = def.clone();
    if let Some(Value::Object(terrain)) = def.get("terrain") {
        next.insert("terrain".to_string(), patch(terrain));
    }
    if let Some(Value::Array(entities)) = def.get("entities") {
        let patched = entities.iter().map(|e| patch_entity(e, patch)).collect();
        next.insert("entities".to_string(), Value::Array(patched));
    }
    Value::Object(next)
}

fn patch_entity(e: &Value, patch: &dyn Fn(&Map<String, Value>) -> Value) -> Value {
    let Value::Object(entity) = e else {
        return e.clone();
    };
    if entity.get("app") != Some(&Value::from("terrain")) {
        return e.clone();
    }
    let mut next = entity.clone();
    if let Some(Value::Object(config)) = entity.get("config") {
        next.insert("config".to_string(), patch(config));
    }
    if let Some(Value::Object(custom)) = entity.get("custom") {
        if finite(custom.get("seed")).is_some() {
            next.insert("custom".to_string(), patch(custom));
        }
    }
    Value::Object(next)
}

mod tests {
    use super::*;

    const TOLERANCE: f64 = 1e-9;

    fn fixture() -> Value {
        serde_json::from_str(include_str!("../testdata/parity/spoint-terrain-config.json")).unwrap()
    }

    fn cases<'a>(f: &'a Value, key: &str) -> &'a Vec<Value> {
        let list = f[key].as_array().unwrap();
        assert!(!list.is_empty(), "{key} has no cases");
        list
    }

    fn close(label: &str, got: f64, want: &Value) {
        match want.as_f64() {
            Some(w) => assert!((got - w).abs() <= TOLERANCE, "{label}: bevy {got} differs from spoint {w}"),
            None => assert!(want.is_null() && got.is_nan(), "{label}: bevy {got} differs from spoint {want}"),
        }
    }

    fn close_opt(label: &str, got: Option<f64>, want: &Value) {
        match got {
            Some(g) => close(label, g, want),
            None => assert!(want.is_null(), "{label}: bevy has no value, spoint {want}"),
        }
    }

    fn carves_match(label: &str, got: &[Carve], want: &Value) {
        let want = want.as_array().unwrap();
        assert_eq!(got.len(), want.len(), "{label}: carve count");
        for (i, (c, w)) in got.iter().zip(want).enumerate() {
            for (k, d) in c.dir.iter().enumerate() {
                close(&format!("{label} carve {i} dir {k}"), *d, &w["dir"][k]);
            }
            close(&format!("{label} carve {i} innerM"), c.inner_m, &w["innerM"]);
            close(&format!("{label} carve {i} outerM"), c.outer_m, &w["outerM"]);
        }
    }

    fn check<T: std::fmt::Debug>(label: &str, got: Result<T, Refusal>, want: &Value, same: impl Fn(&T, &Value)) {
        match (got, want["ok"].as_bool()) {
            (Ok(value), Some(true)) => same(&value, &want["value"]),
            (Err(refusal), Some(false)) => {
                assert_eq!(want["name"].as_str(), Some(refusal.name), "{label}: refusal kind");
                assert_eq!(want["message"].as_str(), Some(refusal.message.as_str()), "{label}: refusal message");
            }
            (got, _) => panic!("{label}: spoint returns {want}, bevy returns {got:?}"),
        }
    }

    fn check_parse(case: &Value) {
        let want = &case["expect"];
        match (parse_terrain_config(&case["tcfg"]), want["ok"].as_bool()) {
            (Ok(cfg), Some(true)) => {
                let w = &want["config"];
                close_opt("parseTerrainConfig seed", cfg.seed, &w["seed"]);
                close_opt("parseTerrainConfig radius", cfg.radius, &w["radius"]);
                close_opt("parseTerrainConfig reliefScale", cfg.relief_scale, &w["reliefScale"]);
                assert_eq!(cfg.anchor_dir, w["anchorDir"], "parseTerrainConfig anchorDir");
                assert_eq!(cfg.center, w["center"], "parseTerrainConfig center");
                assert_eq!(cfg.enabled, w["enabled"].as_bool().unwrap(), "parseTerrainConfig enabled");
                assert_eq!(u64::from(cfg.hash_version), w["hashVersion"].as_u64().unwrap(), "parseTerrainConfig hashVersion");
                carves_match("parseTerrainConfig", &cfg.carves, &w["carves"]);
                assert_eq!(cfg.carve_inputs, w["carveInputs"], "parseTerrainConfig carveInputs");
                close("parseTerrainConfig extent", cfg.extent, &w["extent"]);
                close("parseTerrainConfig res", cfg.res, &w["res"]);
            }
            (Err(refusal), Some(false)) => {
                assert_eq!(want["path"].as_str(), Some(refusal.path), "parseTerrainConfig refusal path");
                assert_eq!(want["reason"].as_str(), Some(refusal.reason.as_str()), "parseTerrainConfig refusal reason");
            }
            (got, _) => panic!("parseTerrainConfig: spoint returns {want}, bevy returns {got:?}"),
        }
    }

    #[test]
    fn terrain_config_matches_spoint() {
        let f = fixture();
        for case in cases(&f, "hashVersionOf") {
            check("terrainHashVersionOf", terrain_hash_version_of(&case["tcfg"]), &case["expect"], |v: &u32, w: &Value| {
                assert_eq!(w.as_u64(), Some(u64::from(*v)), "terrainHashVersionOf value");
            });
        }
        for case in cases(&f, "assertHashVersion") {
            check("assertTerrainHashVersion", assert_terrain_hash_version(&case["v"]), &case["expect"], |v: &u32, w: &Value| {
                assert_eq!(w.as_u64(), Some(u64::from(*v)), "assertTerrainHashVersion value");
            });
        }
        for case in cases(&f, "hashOverride") {
            check("parseTerrainHashOverride", parse_terrain_hash_override(case["raw"].as_str()), &case["expect"], |v: &Option<u32>, w: &Value| match v {
                None => assert!(w.is_null(), "parseTerrainHashOverride null"),
                Some(h) => assert_eq!(w.as_u64(), Some(u64::from(*h)), "parseTerrainHashOverride value"),
            });
        }
        for case in cases(&f, "carvesOf") {
            check("terrainCarvesOf", terrain_carves_of(&case["tcfg"]), &case["expect"], |got: &Vec<Carve>, want: &Value| {
                carves_match("terrainCarvesOf", got, want);
            });
        }
        for case in cases(&f, "resolve") {
            check("resolveTerrainConfig", Ok(resolve_terrain_config(&case["worldDef"])), &case["expect"], |v: &Value, w: &Value| {
                assert_eq!(v, w, "resolveTerrainConfig value");
            });
        }
        for case in cases(&f, "parse") {
            check_parse(case);
        }
    }

    fn bake_fixture() -> Value {
        serde_json::from_str(include_str!("../testdata/parity/spoint-terrain-bake.json")).unwrap()
    }

    #[test]
    fn terrain_bake_matches_spoint() {
        let f = bake_fixture();
        for case in cases(&f, "minimapDescriptor") {
            check("minimapDescriptor", minimap_descriptor(&case["worldId"], &case["tcfg"]), &case["expect"], |got: &Option<MinimapDescriptor>, want: &Value| match got {
                None => assert!(want.is_null(), "minimapDescriptor null"),
                Some(d) => {
                    assert_eq!(d.base, want["base"].as_str().unwrap(), "minimapDescriptor base");
                    assert_eq!(canonical_json(&d.center), canonical_json(&want["center"]), "minimapDescriptor center");
                    close("minimapDescriptor extent", d.extent, &want["extent"]);
                }
            });
        }
        for case in cases(&f, "minimapBakeParams") {
            check("minimapBakeParams", minimap_bake_params(&case["tcfg"]), &case["expect"], |p: &MinimapBakeParams, want: &Value| {
                close_opt("minimapBakeParams radius", p.radius, &want["radius"]);
                close_opt("minimapBakeParams reliefScale", p.relief_scale, &want["reliefScale"]);
                assert_eq!(u64::from(p.hash_version), want["hashVersion"].as_u64().unwrap(), "minimapBakeParams hashVersion");
                carves_match("minimapBakeParams", &p.carves, &want["carves"]);
                assert_eq!(canonical_json(&p.anchor_dir), canonical_json(&want["anchorDir"]), "minimapBakeParams anchorDir");
                close("minimapBakeParams extent", p.extent, &want["extent"]);
                close("minimapBakeParams res", p.res, &want["res"]);
                assert_eq!(canonical_json(&p.center), canonical_json(&want["center"]), "minimapBakeParams center");
            });
        }
        for case in cases(&f, "terrainBakeKey") {
            check("terrainBakeKey", terrain_bake_key(&case["tcfg"]), &case["expect"], |key: &String, want: &Value| {
                assert_eq!(key.as_str(), want.as_str().unwrap(), "terrainBakeKey");
            });
        }
        for case in cases(&f, "withTerrainSeed") {
            check("withTerrainSeed", with_terrain_seed(&case["worldDef"], &case["seed"]).map(|v| canonical_json(&v)), &case["expect"], |text: &String, want: &Value| {
                assert_eq!(text.as_str(), want.as_str().unwrap(), "withTerrainSeed");
            });
        }
    }
}
