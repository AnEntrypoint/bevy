pub const FNV1A_32_OFFSET_BASIS: u32 = 0x811c_9dc5;
pub const FNV1A_32_PRIME: u32 = 0x0100_0193;

fn step(hash: u32, unit: u32) -> u32 {
    (hash ^ unit).wrapping_mul(FNV1A_32_PRIME)
}

pub fn step_string(hash: u32, s: &str) -> u32 {
    s.encode_utf16().fold(hash, |h, unit| step(h, u32::from(unit)))
}

pub fn step_bytes(hash: u32, bytes: &[u8]) -> u32 {
    bytes.iter().fold(hash, |h, byte| step(h, u32::from(*byte)))
}

pub fn step_float32(hash: u32, n: f64) -> u32 {
    step_bytes(hash, &(n as f32).to_be_bytes())
}

pub fn step_float64(hash: u32, n: f64, byte_offset: u32) -> u32 {
    n.to_be_bytes().iter().fold(hash, |h, byte| step(h, u32::from(*byte).wrapping_add(byte_offset)))
}

pub fn fnv1a_string(s: &str) -> u32 {
    step_string(FNV1A_32_OFFSET_BASIS, s)
}

pub fn fnv1a_bytes(bytes: &[u8]) -> u32 {
    step_bytes(FNV1A_32_OFFSET_BASIS, bytes)
}

mod tests {
    use super::*;
    use serde_json::Value;

    fn u32_of(v: &Value) -> u32 {
        u32::try_from(v.as_u64().unwrap()).unwrap()
    }

    fn bytes_of(v: &Value) -> Vec<u8> {
        v.as_array().unwrap().iter().map(|b| u8::try_from(b.as_u64().unwrap()).unwrap()).collect()
    }

    #[test]
    fn fnv1a_matches_spoint() {
        let f: Value = crate::testkit::parse(include_str!("../testdata/parity/spoint-fnv1a.json"));
        assert_eq!(u64::from(FNV1A_32_OFFSET_BASIS), f["offsetBasis"].as_u64().unwrap(), "offset basis");
        assert_eq!(u64::from(FNV1A_32_PRIME), f["prime"].as_u64().unwrap(), "prime");
        for case in crate::testkit::cases(&f, "string") {
            let input = case["input"].as_str().unwrap();
            assert_eq!(Some(u64::from(fnv1a_string(input))), case["expect"].as_u64(), "fnv1aString {input:?}");
        }
        for case in crate::testkit::cases(&f, "bytes") {
            assert_eq!(Some(u64::from(fnv1a_bytes(&bytes_of(&case["input"])))), case["expect"].as_u64(), "fnv1aBytes {}", case["input"]);
        }
        for case in crate::testkit::cases(&f, "stepString") {
            let out = step_string(u32_of(&case["hash"]), case["input"].as_str().unwrap());
            assert_eq!(Some(u64::from(out)), case["expect"].as_u64(), "stepString {}", case["input"]);
        }
        for case in crate::testkit::cases(&f, "stepBytes") {
            let out = step_bytes(u32_of(&case["hash"]), &bytes_of(&case["input"]));
            assert_eq!(Some(u64::from(out)), case["expect"].as_u64(), "stepBytes {}", case["input"]);
        }
        for case in crate::testkit::cases(&f, "stepFloat32") {
            let out = step_float32(u32_of(&case["hash"]), case["input"].as_f64().unwrap());
            assert_eq!(Some(u64::from(out)), case["expect"].as_u64(), "stepFloat32 {}", case["input"]);
        }
        for case in crate::testkit::cases(&f, "stepFloat64") {
            let out = step_float64(u32_of(&case["hash"]), case["input"].as_f64().unwrap(), u32_of(&case["byteOffset"]));
            assert_eq!(Some(u64::from(out)), case["expect"].as_u64(), "stepFloat64 {}", case["input"]);
        }
    }
}
