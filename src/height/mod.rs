mod height_spec;
mod ops_jsgen;

#[cfg(test)]
mod tests {
    use super::height_spec::{Carve, HashVersion, HeightSpec, Params};
    use super::ops_jsgen::{Precision, V3};
    use serde_json::Value;

    const NO_PARAMS: Params = Params { hpf_res: 8.0, land_bias: 0.0, beach_shelf_m: 0.0, relief_scale: 0.0 };

    fn fixture() -> Value {
        serde_json::from_str(include_str!("../../testdata/parity/spoint-height-spec.json")).unwrap()
    }

    fn precision(value: &Value) -> Precision {
        match value.as_str().unwrap() {
            "f32" => Precision::F32,
            "f64" => Precision::F64,
            other => panic!("precision {other}"),
        }
    }

    fn hash(value: &Value) -> HashVersion {
        match value.as_u64().unwrap() {
            1 => HashVersion::Float,
            2 => HashVersion::Integer,
            other => panic!("hashVersion {other}"),
        }
    }

    fn number(value: &Value) -> f64 {
        value.as_f64().unwrap()
    }

    fn vec3(value: &Value) -> V3 {
        let a = value.as_array().unwrap();
        [number(&a[0]), number(&a[1]), number(&a[2])]
    }

    fn numbers(value: &Value) -> Vec<f64> {
        value.as_array().unwrap().iter().map(number).collect()
    }

    fn faces(value: &Value) -> Vec<Vec<f32>> {
        value.as_array().unwrap().iter().map(|face| face.as_array().unwrap().iter().map(|v| number(v) as f32).collect()).collect()
    }

    fn check(name: &str, actual: &[f64], expected: &[f64]) {
        assert_eq!(actual.len(), expected.len(), "{name} count");
        let differing: Vec<String> = actual
            .iter()
            .zip(expected)
            .enumerate()
            .filter(|(_, (a, e))| a != e)
            .map(|(i, (a, e))| format!("{name}[{i}] bevy {a:?} spoint {e:?}"))
            .collect();
        assert!(
            differing.is_empty(),
            "{} of {} differ from spoint: {}",
            differing.len(),
            expected.len(),
            differing.iter().take(3).cloned().collect::<Vec<_>>().join("; ")
        );
    }

    #[test]
    fn snoise3_matches_the_spoint_fixture() {
        let fixture = fixture();
        let mut actual = Vec::new();
        let mut expected = Vec::new();
        for case in fixture["snoise3"].as_array().unwrap() {
            let spec = HeightSpec::new(hash(&case["hashVersion"]), precision(&case["precision"]), NO_PARAMS, Vec::new(), &[]).unwrap();
            actual.extend(case["points"].as_array().unwrap().iter().map(|p| spec.snoise3(vec3(p))));
            expected.extend(numbers(&case["expected"]));
        }
        check("snoise3", &actual, &expected);
    }

    #[test]
    fn compose_height_matches_the_spoint_fixture() {
        let fixture = fixture();
        let hpf = faces(&fixture["hpfFaces"]);
        let mut actual = Vec::new();
        let mut expected = Vec::new();
        for case in fixture["compose"].as_array().unwrap() {
            let params = &case["params"];
            let params = Params {
                hpf_res: number(&params["hpfRes"]),
                land_bias: number(&params["landBias"]),
                beach_shelf_m: number(&params["beachShelfM"]),
                relief_scale: number(&params["reliefScale"]),
            };
            let carves = case["carves"].as_array().unwrap().iter().map(|c| Carve {
                dir: vec3(&c["dir"]),
                inner_chord2: number(&c["innerChord2"]),
                outer_chord2: number(&c["outerChord2"]),
                target_h: number(&c["targetH"]),
            }).collect();
            let spec = HeightSpec::new(hash(&case["hashVersion"]), precision(&case["precision"]), params, carves, &hpf).unwrap();
            actual.extend(case["dirs"].as_array().unwrap().iter().map(|d| spec.compose(vec3(d))));
            expected.extend(numbers(&case["expected"]));
        }
        check("compose", &actual, &expected);
    }
}
