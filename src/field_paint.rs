#![cfg_attr(not(test), allow(dead_code))]

use serde_json::{Map, Value};

use crate::field_layers::{chunk_key, FieldError, FieldLayers};

pub const TERRAIN_FIELD_NAMES: [&str; 3] = ["terrain:height", "terrain:water", "terrain:natural-mask"];
pub const FIELD_LAYER_NAMES: [&str; 2] = ["PLAYER", "CREATOR"];
pub const PAINTED_HEIGHT_FIELD: &str = "terrain:height";
pub const NATURAL_MASK_FIELD: &str = "terrain:natural-mask";

const DEFAULT_CELL_SIZE: f64 = 1.0;
const DEFAULT_CHUNK_CELLS: f64 = 64.0;

fn refusal(code: &'static str, message: &str) -> FieldError {
    FieldError {
        reason_code: Some(code),
        message: format!("[field-paint] {code}: {message}"),
    }
}

fn field_index(field: &str) -> Result<usize, FieldError> {
    TERRAIN_FIELD_NAMES
        .iter()
        .position(|name| *name == field)
        .ok_or_else(|| {
            refusal(
                "field-unknown",
                &format!("field \"{field}\" is not one of {}", TERRAIN_FIELD_NAMES.join(", ")),
            )
        })
}

fn layer_index(layer: &str) -> Result<usize, FieldError> {
    FIELD_LAYER_NAMES
        .iter()
        .position(|name| *name == layer)
        .ok_or_else(|| {
            refusal(
                "layer-unknown",
                &format!("layer \"{layer}\" is not one of {}", FIELD_LAYER_NAMES.join(", ")),
            )
        })
}

fn slot(field: &str, layer: &str) -> Result<usize, FieldError> {
    Ok(field_index(field)? * FIELD_LAYER_NAMES.len() + layer_index(layer)?)
}

#[derive(Debug, Clone, Copy)]
pub struct StrokeSpec<'a> {
    pub field: &'a str,
    pub layer: &'a str,
    pub center: [f64; 2],
    pub place: Option<[f64; 2]>,
    pub radius: f64,
}

pub struct ChunkInfo {
    pub cell_size: f64,
    pub chunk_cells: f64,
    pub key: String,
    pub painted: bool,
    pub version: u64,
}

struct Painted {
    x: f64,
    z: f64,
    delta: f64,
}

struct AppliedStroke {
    slot: usize,
    cells: Vec<Painted>,
}

pub struct FieldPaint {
    cell_size: f64,
    chunk_cells: f64,
    grids: Vec<FieldLayers>,
    history: Vec<AppliedStroke>,
}

impl FieldPaint {
    pub fn new(cell_size: f64, chunk_cells: f64) -> Result<Self, FieldError> {
        let grids = (0..TERRAIN_FIELD_NAMES.len() * FIELD_LAYER_NAMES.len())
            .map(|_| FieldLayers::new(cell_size, chunk_cells))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            cell_size,
            chunk_cells,
            grids,
            history: Vec::new(),
        })
    }

    pub fn stroke(
        &mut self,
        spec: StrokeSpec<'_>,
        fill: impl Fn([f64; 2], f64, f64) -> f64,
    ) -> Result<usize, FieldError> {
        let grid_slot = slot(spec.field, spec.layer)?;
        if !spec.center.iter().all(|value| value.is_finite()) {
            return Err(refusal("center-invalid", "center must be [x, z] of finite numbers"));
        }
        let place = spec.place.unwrap_or([0.0, 0.0]);
        if !place.iter().all(|value| value.is_finite()) {
            return Err(refusal("place-invalid", "place must be [x, z] of finite numbers"));
        }
        if !(spec.radius > 0.0) {
            return Err(refusal("radius-invalid", "radius must be a positive number"));
        }
        let origin = [spec.center[0] + place[0], spec.center[1] + place[1]];
        let cell = self.cell_size;
        let grid = &mut self.grids[grid_slot];
        let mut cells = Vec::new();
        let mut x = origin[0] - spec.radius;
        while x <= origin[0] + spec.radius {
            let mut z = origin[1] - spec.radius;
            while z <= origin[1] + spec.radius {
                let distance = (x - origin[0]).hypot(z - origin[1]);
                if distance <= spec.radius {
                    let current = grid.read_field(x, z);
                    let delta = fill([x, z], 1.0 - distance / spec.radius, current) - current;
                    if delta != 0.0 {
                        grid.paint(x, z, delta);
                        cells.push(Painted { x, z, delta });
                    }
                }
                z += cell;
            }
            x += cell;
        }
        let count = cells.len();
        self.history.push(AppliedStroke {
            slot: grid_slot,
            cells,
        });
        Ok(count)
    }

    pub fn undo(&mut self) -> bool {
        let Some(last) = self.history.pop() else {
            return false;
        };
        let grid = &mut self.grids[last.slot];
        for cell in &last.cells {
            grid.paint(cell.x, cell.z, -cell.delta);
        }
        true
    }

    pub fn read(&self, field: &str, x: f64, z: f64) -> Result<f64, FieldError> {
        let base = field_index(field)? * FIELD_LAYER_NAMES.len();
        let mut sum = 0.0;
        for layer in 0..FIELD_LAYER_NAMES.len() {
            sum += self.grids[base + layer].read_field(x, z);
        }
        Ok(sum)
    }

    pub fn chunk_object(&self, field: &str, layer: &str, kx: i64, kz: i64) -> Result<ChunkInfo, FieldError> {
        let grid = &self.grids[slot(field, layer)?];
        let key = chunk_key(kx, kz);
        Ok(ChunkInfo {
            cell_size: self.cell_size,
            chunk_cells: self.chunk_cells,
            painted: grid.is_painted(&key),
            key,
            version: grid.version(),
        })
    }

    pub fn to_json(&self) -> Value {
        let mut out = Map::new();
        for (field_pos, field) in TERRAIN_FIELD_NAMES.iter().enumerate() {
            let mut layers = Map::new();
            for (layer_pos, layer) in FIELD_LAYER_NAMES.iter().enumerate() {
                let grid = &self.grids[field_pos * FIELD_LAYER_NAMES.len() + layer_pos];
                layers.insert(layer.to_string(), grid.export_json());
            }
            out.insert(field.to_string(), Value::Object(layers));
        }
        Value::Object(out)
    }

    pub fn from_json(&mut self, doc: &Value) -> Result<(), FieldError> {
        let Some(fields) = doc.as_object() else {
            return Err(refusal(
                "paint-doc-invalid",
                "paint document must be an object keyed by field name",
            ));
        };
        for (name, layers) in fields {
            let field = TERRAIN_FIELD_NAMES
                .iter()
                .position(|known| *known == name.as_str())
                .ok_or_else(|| {
                    refusal(
                        "field-unknown",
                        &format!(
                            "saved field \"{name}\" is not one of {}",
                            TERRAIN_FIELD_NAMES.join(", ")
                        ),
                    )
                })?;
            for (layer, list) in layers.as_object().into_iter().flat_map(|map| map.iter()) {
                let index = field * FIELD_LAYER_NAMES.len() + layer_index(layer)?;
                self.grids[index].import_runtime(list)?;
            }
        }
        Ok(())
    }
}

pub fn wrap_height_with_field_paint(
    paint: Option<&FieldPaint>,
    x: f64,
    z: f64,
    y_guess: f64,
    height: impl Fn(f64, f64, f64) -> f64,
) -> Result<f64, FieldError> {
    let base = height(x, z, y_guess);
    match paint {
        None => Ok(base),
        Some(paint) => Ok(base + paint.read(PAINTED_HEIGHT_FIELD, x, z)?),
    }
}

pub fn natural_mask_at(paint: Option<&FieldPaint>, x: f64, z: f64) -> Result<f64, FieldError> {
    match paint {
        Some(paint) => paint.read(NATURAL_MASK_FIELD, x, z),
        None => Err(refusal("paint-invalid", "paint must be a createFieldPaint instance")),
    }
}

pub fn create_host_field_paint(terrain_cfg: Option<&Value>) -> Result<FieldPaint, FieldError> {
    let Some(config) = terrain_cfg.and_then(|cfg| cfg.get("fieldPaint")) else {
        return FieldPaint::new(DEFAULT_CELL_SIZE, DEFAULT_CHUNK_CELLS);
    };
    if !(config.is_object() || config.is_array()) {
        return Err(refusal(
            "field-paint-config-invalid",
            "terrain.fieldPaint must be an object with cellSize and chunkCells",
        ));
    }
    let cell_size = match config.get("cellSize") {
        None | Some(Value::Null) => DEFAULT_CELL_SIZE,
        Some(value) => value
            .as_f64()
            .filter(|size| size.is_finite() && *size > 0.0)
            .ok_or_else(|| {
                refusal(
                    "field-paint-cell-size-invalid",
                    "terrain.fieldPaint.cellSize must be a positive finite number",
                )
            })?,
    };
    let chunk_cells = match config.get("chunkCells") {
        None | Some(Value::Null) => DEFAULT_CHUNK_CELLS,
        Some(value) => value
            .as_f64()
            .filter(|cells| cells.fract() == 0.0 && *cells >= 1.0)
            .ok_or_else(|| {
                refusal(
                    "field-paint-chunk-cells-invalid",
                    "terrain.fieldPaint.chunkCells must be an integer of at least 1",
                )
            })?,
    };
    FieldPaint::new(cell_size, chunk_cells)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::BTreeSet;

    const FIXTURE: &str = include_str!("../testdata/parity/spoint-field-paint.json");
    const TOLERANCE: f64 = 1e-6;

    fn fixture() -> Value {
        serde_json::from_str(FIXTURE).unwrap()
    }

    fn num(value: &Value) -> f64 {
        match value {
            Value::String(text) => text.parse().unwrap(),
            other => other.as_f64().unwrap(),
        }
    }

    fn int(value: &Value) -> i64 {
        value.as_i64().unwrap()
    }

    fn point(step: &Value) -> (f64, f64) {
        (num(&step["x"]), num(&step["z"]))
    }

    fn cells(value: &Value) -> Vec<f32> {
        value.as_array().unwrap().iter().map(|cell| num(cell) as f32).collect()
    }

    fn assert_json_close(got: &Value, want: &Value, label: &str) {
        match (got, want) {
            (Value::Object(g), Value::Object(w)) => {
                assert_eq!(
                    g.keys().collect::<BTreeSet<_>>(),
                    w.keys().collect::<BTreeSet<_>>(),
                    "{label}: keys"
                );
                for (key, want_value) in w {
                    assert_json_close(&g[key], want_value, &format!("{label}.{key}"));
                }
            }
            (Value::Array(g), Value::Array(w)) => {
                assert_eq!(g.len(), w.len(), "{label}: length");
                for (index, (g_value, w_value)) in g.iter().zip(w).enumerate() {
                    assert_json_close(g_value, w_value, &format!("{label}[{index}]"));
                }
            }
            (Value::Number(_), Value::Number(_)) => {
                let (g, w) = (got.as_f64().unwrap(), want.as_f64().unwrap());
                assert!((g - w).abs() <= TOLERANCE, "{label}: got {g} want {w}");
            }
            _ => assert_eq!(got, want, "{label}"),
        }
    }

    fn expect(label: &str, want: &Value, got: Result<Value, FieldError>) {
        match got {
            Ok(value) => assert_json_close(&value, &want["result"], label),
            Err(err) => assert_json_close(
                &json!({ "reason": err.reason_code, "message": err.message }),
                &want["error"],
                label,
            ),
        }
    }

    fn stroke_value(step: &Value, pos: [f64; 2], inset: f64, current: f64) -> f64 {
        match step["kind"].as_str().unwrap() {
            "add" => current + num(&step["amount"]) * inset,
            "set" => current + (num(&step["target"]) - current) * inset,
            "slope" => current + inset * (num(&step["ax"]) * pos[0] + num(&step["az"]) * pos[1]),
            other => panic!("unknown stroke kind {other}"),
        }
    }

    fn height(x: f64, z: f64, y_guess: f64) -> f64 {
        1.5 + 0.5 * x - 0.25 * z + 0.125 * y_guess
    }

    fn run_paint_step(paint: &mut FieldPaint, step: &Value) -> Result<Value, FieldError> {
        match step["op"].as_str().unwrap() {
            "stroke" => {
                let spec = StrokeSpec {
                    field: step["field"].as_str().unwrap(),
                    layer: step["layer"].as_str().unwrap(),
                    center: [num(&step["center"][0]), num(&step["center"][1])],
                    place: step.get("place").map(|place| [num(&place[0]), num(&place[1])]),
                    radius: num(&step["radius"]),
                };
                paint
                    .stroke(spec, |pos, inset, current| stroke_value(step, pos, inset, current))
                    .map(|count| json!(count))
            }
            "undo" => Ok(json!(paint.undo())),
            "read" => {
                let (x, z) = point(step);
                paint.read(step["field"].as_str().unwrap(), x, z).map(|value| json!(value))
            }
            "chunkObject" => paint
                .chunk_object(
                    step["field"].as_str().unwrap(),
                    step["layer"].as_str().unwrap(),
                    int(&step["kx"]),
                    int(&step["kz"]),
                )
                .map(|info| {
                    json!({
                        "cellSize": info.cell_size,
                        "chunkCells": info.chunk_cells,
                        "key": info.key,
                        "painted": info.painted,
                        "version": info.version
                    })
                }),
            "toJSON" => Ok(paint.to_json()),
            "fromJSON" => paint.from_json(&step["input"]).map(|()| Value::Null),
            "wrap" => {
                let (x, z) = point(step);
                wrap_height_with_field_paint(Some(&*paint), x, z, num(&step["yGuess"]), height)
                    .map(|value| json!(value))
            }
            "wrapNull" => {
                let (x, z) = point(step);
                wrap_height_with_field_paint(None, x, z, num(&step["yGuess"]), height)
                    .map(|value| json!(value))
            }
            "naturalMask" => {
                let (x, z) = point(step);
                natural_mask_at(Some(&*paint), x, z).map(|value| json!(value))
            }
            "naturalMaskNull" => {
                let (x, z) = point(step);
                natural_mask_at(None, x, z).map(|value| json!(value))
            }
            other => panic!("unknown paint op {other}"),
        }
    }

    fn run_layer_step(layer: &mut FieldLayers, step: &Value) -> Result<Value, FieldError> {
        match step["op"].as_str().unwrap() {
            "span" => Ok(json!(layer.span())),
            "locate" => {
                let (x, z) = point(step);
                let (key, index) = layer.locate(x, z);
                Ok(json!({ "key": key, "index": index }))
            }
            "setAuthored" => layer
                .set_authored(int(&step["kx"]), int(&step["kz"]), cells(&step["cells"]))
                .map(|()| Value::Null),
            "paint" => {
                let (x, z) = point(step);
                Ok(json!(layer.paint(x, z, num(&step["delta"]))))
            }
            "readField" => {
                let (x, z) = point(step);
                Ok(json!(layer.read_field(x, z)))
            }
            "changes" => {
                let changes = layer.changes(step["at"].as_u64().unwrap());
                Ok(json!({ "chunks": changes.chunks, "at": changes.at }))
            }
            "clearRuntime" => Ok(json!(layer.clear_runtime(int(&step["kx"]), int(&step["kz"])))),
            "clearAllRuntime" => Ok(json!(layer.clear_all_runtime())),
            "importRuntime" => layer.import_runtime(&step["list"]).map(|count| json!(count)),
            "paintedChunks" => Ok(json!(layer.painted_chunks())),
            "exportRuntime" => Ok(layer.export_json()),
            other => panic!("unknown layer op {other}"),
        }
    }

    fn run_paint_trace(trace: &Value, compare: &[&str]) {
        let fixture = fixture();
        let mut paint = FieldPaint::new(num(&fixture["cellSize"]), num(&fixture["chunkCells"])).unwrap();
        for step in trace.as_array().unwrap() {
            let op = step["op"].as_str().unwrap();
            let got = run_paint_step(&mut paint, step);
            if compare.contains(&op) {
                expect(&format!("paint {op}"), step, got);
            }
        }
    }

    #[test]
    fn field_paint_layers_match_spoint_create_field_layers() {
        let fixture = fixture();
        let mut layer = FieldLayers::new(num(&fixture["cellSize"]), num(&fixture["chunkCells"])).unwrap();
        for step in fixture["layerTrace"].as_array().unwrap() {
            let got = run_layer_step(&mut layer, step);
            expect(&format!("layer {}", step["op"].as_str().unwrap()), step, got);
        }
        for case in fixture["layerConstructErrors"].as_array().unwrap() {
            let got = FieldLayers::new(num(&case["cellSize"]), num(&case["chunkCells"])).map(|_| Value::Null);
            expect("layer construct", case, got);
        }
    }

    #[test]
    fn field_paint_strokes_reads_and_undo_match_spoint_create_field_paint() {
        run_paint_trace(
            &fixture()["paintTrace"],
            &["stroke", "read", "undo", "chunkObject", "toJSON"],
        );
    }

    #[test]
    fn field_paint_restored_round_trip_matches_spoint_from_json() {
        run_paint_trace(&fixture()["restoredTrace"], &["fromJSON", "toJSON", "read", "undo"]);
    }

    #[test]
    fn field_paint_construct_refusals_match_spoint() {
        let fixture = fixture();
        for case in fixture["paintConstructErrors"].as_array().unwrap() {
            let got = FieldPaint::new(num(&case["cellSize"]), num(&case["chunkCells"])).map(|_| Value::Null);
            expect("paint construct", case, got);
        }
    }

    #[test]
    fn field_paint_names_match_spoint() {
        let fixture = fixture();
        assert_json_close(&json!(TERRAIN_FIELD_NAMES), &fixture["fieldNames"], "fieldNames");
        assert_json_close(&json!(FIELD_LAYER_NAMES), &fixture["layerNames"], "layerNames");
        assert_json_close(&json!(PAINTED_HEIGHT_FIELD), &fixture["paintedHeightField"], "paintedHeightField");
        assert_json_close(&json!(NATURAL_MASK_FIELD), &fixture["naturalMaskField"], "naturalMaskField");
    }

    #[test]
    fn field_paint_natural_mask_at_matches_spoint() {
        run_paint_trace(&fixture()["paintTrace"], &["naturalMask", "naturalMaskNull"]);
    }

    #[test]
    fn field_paint_wrap_height_matches_spoint() {
        run_paint_trace(&fixture()["paintTrace"], &["wrap", "wrapNull"]);
    }

    #[test]
    fn field_paint_host_defaults_and_refusals_match_spoint() {
        let fixture = fixture();
        for case in fixture["hostCases"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let config = if case.get("absent").is_some() {
                None
            } else {
                Some(&case["input"])
            };
            let got = create_host_field_paint(config).and_then(|paint| {
                let info = paint.chunk_object(PAINTED_HEIGHT_FIELD, FIELD_LAYER_NAMES[0], 0, 0)?;
                Ok(json!({ "cellSize": info.cell_size, "chunkCells": info.chunk_cells }))
            });
            expect(&format!("host {name}"), case, got);
        }
    }
}
