
use std::collections::BTreeMap;

use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct FieldError {
    pub reason_code: Option<&'static str>,
    pub message: String,
}

pub struct Changes {
    pub chunks: Vec<String>,
    pub at: u64,
}

pub struct FieldLayers {
    cell_size: f64,
    chunk_cells: usize,
    authored: BTreeMap<String, Vec<f32>>,
    runtime: BTreeMap<String, Vec<f32>>,
    versions: BTreeMap<String, u64>,
    version: u64,
}

pub fn chunk_key(kx: i64, kz: i64) -> String {
    format!("{kx},{kz}")
}

impl FieldLayers {
    pub fn new(cell_size: f64, chunk_cells: f64) -> Result<Self, FieldError> {
        if !(cell_size > 0.0) || chunk_cells.fract() != 0.0 || chunk_cells < 1.0 {
            return Err(FieldError {
                reason_code: None,
                message: "FieldLayers: cellSize > 0 and integer chunkCells >= 1 required".to_string(),
            });
        }
        Ok(Self {
            cell_size,
            chunk_cells: chunk_cells as usize,
            authored: BTreeMap::new(),
            runtime: BTreeMap::new(),
            versions: BTreeMap::new(),
            version: 0,
        })
    }

    fn cells_per_chunk(&self) -> usize {
        self.chunk_cells * self.chunk_cells
    }

    fn bump(&mut self, key: String) {
        self.version += 1;
        self.versions.insert(key, self.version);
    }

    pub fn span(&self) -> f64 {
        self.cell_size * self.chunk_cells as f64
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn locate(&self, x: f64, z: f64) -> (String, usize) {
        let n = self.chunk_cells as i64;
        let gx = (x / self.cell_size).floor() as i64;
        let gz = (z / self.cell_size).floor() as i64;
        let kx = gx.div_euclid(n);
        let kz = gz.div_euclid(n);
        let ix = gx - kx * n;
        let iz = gz - kz * n;
        (chunk_key(kx, kz), (iz * n + ix) as usize)
    }

    pub fn read_field(&self, x: f64, z: f64) -> f64 {
        let (key, index) = self.locate(x, z);
        let authored = self.authored.get(&key).map_or(0.0, |cells| f64::from(cells[index]));
        let runtime = self.runtime.get(&key).map_or(0.0, |cells| f64::from(cells[index]));
        authored + runtime
    }

    pub fn paint(&mut self, x: f64, z: f64, delta: f64) -> f64 {
        let (key, index) = self.locate(x, z);
        let per_chunk = self.cells_per_chunk();
        let cells = self.runtime.entry(key.clone()).or_insert_with(|| vec![0.0; per_chunk]);
        cells[index] = (f64::from(cells[index]) + delta) as f32;
        let value = f64::from(cells[index]);
        self.bump(key);
        value
    }

    pub fn set_authored(&mut self, kx: i64, kz: i64, cells: Vec<f32>) -> Result<(), FieldError> {
        if cells.len() != self.cells_per_chunk() {
            return Err(FieldError {
                reason_code: None,
                message: "FieldLayers: authored chunk must be a Float32Array of chunkCells^2".to_string(),
            });
        }
        let key = chunk_key(kx, kz);
        self.authored.insert(key.clone(), cells);
        self.bump(key);
        Ok(())
    }

    pub fn clear_runtime(&mut self, kx: i64, kz: i64) -> bool {
        let key = chunk_key(kx, kz);
        if self.runtime.remove(&key).is_none() {
            return false;
        }
        self.bump(key);
        true
    }

    pub fn clear_all_runtime(&mut self) -> usize {
        let cleared = std::mem::take(&mut self.runtime);
        let count = cleared.len();
        for key in cleared.into_keys() {
            self.bump(key);
        }
        count
    }

    pub fn changes(&self, at: u64) -> Changes {
        Changes {
            chunks: self
                .versions
                .iter()
                .filter(|(_, version)| **version > at)
                .map(|(key, _)| key.clone())
                .collect(),
            at: self.version,
        }
    }

    pub fn painted_chunks(&self) -> Vec<String> {
        self.runtime.keys().cloned().collect()
    }

    pub fn is_painted(&self, key: &str) -> bool {
        self.runtime.contains_key(key)
    }

    pub fn export_json(&self) -> Value {
        Value::Array(
            self.runtime
                .iter()
                .map(|(key, cells)| {
                    json!({ "key": key, "cells": cells.iter().map(|cell| f64::from(*cell)).collect::<Vec<f64>>() })
                })
                .collect(),
        )
    }

    pub fn import_runtime(&mut self, list: &Value) -> Result<usize, FieldError> {
        let Some(entries) = list.as_array() else {
            return Err(FieldError {
                reason_code: None,
                message: "FieldLayers: importRuntime needs an array of { key, cells }".to_string(),
            });
        };
        let per_chunk = self.cells_per_chunk();
        for entry in entries {
            let key = entry.get("key").and_then(Value::as_str);
            let cells = entry
                .get("cells")
                .and_then(Value::as_array)
                .filter(|cells| cells.len() == per_chunk);
            let (Some(key), Some(cells)) = (key, cells) else {
                return Err(FieldError {
                    reason_code: None,
                    message: "FieldLayers: runtime chunk must carry chunkCells^2 cells".to_string(),
                });
            };
            let values: Option<Vec<f64>> = cells.iter().map(Value::as_f64).collect();
            let Some(values) = values.filter(|values| values.iter().all(|value| value.is_finite())) else {
                return Err(FieldError {
                    reason_code: None,
                    message: "FieldLayers: runtime chunk cells must be finite numbers".to_string(),
                });
            };
            self.runtime.insert(key.to_string(), values.iter().map(|value| *value as f32).collect());
            self.bump(key.to_string());
        }
        Ok(entries.len())
    }
}
