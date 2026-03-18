/// Parsed 3D LUT data.
#[derive(Debug, Clone)]
pub struct Lut3D {
    pub size: u32,
    pub data: Vec<[f32; 3]>,
}

/// Parse a `.cube` 3D LUT file.
///
/// Supports the standard `.cube` format:
/// - `LUT_3D_SIZE N` declares the grid size
/// - Lines starting with `#` are comments
/// - `TITLE` lines are ignored
/// - `DOMAIN_MIN` and `DOMAIN_MAX` lines are ignored (assumed 0..1)
/// - Data lines are `R G B` float triplets, one per line
/// - Total entries must equal `size^3`
pub fn parse_cube(content: &str) -> Result<Lut3D, String> {
    let mut size: Option<u32> = None;
    let mut data: Vec<[f32; 3]> = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Skip TITLE lines
        if trimmed.starts_with("TITLE") {
            continue;
        }

        // Skip DOMAIN_MIN / DOMAIN_MAX
        if trimmed.starts_with("DOMAIN_MIN") || trimmed.starts_with("DOMAIN_MAX") {
            continue;
        }

        // Parse LUT_3D_SIZE
        if trimmed.starts_with("LUT_3D_SIZE") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() < 2 {
                return Err("LUT_3D_SIZE missing value".into());
            }
            let n: u32 = parts[1]
                .parse()
                .map_err(|e| format!("invalid LUT_3D_SIZE value: {e}"))?;
            if !(2..=256).contains(&n) {
                return Err(format!("LUT_3D_SIZE {n} out of range [2, 256]"));
            }
            size = Some(n);
            continue;
        }

        // Skip LUT_1D_SIZE (unsupported)
        if trimmed.starts_with("LUT_1D_SIZE") {
            return Err("1D LUTs are not supported, only 3D".into());
        }

        // Parse data triplet
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() >= 3 {
            let r: f32 = parts[0]
                .parse()
                .map_err(|e| format!("invalid R value: {e}"))?;
            let g: f32 = parts[1]
                .parse()
                .map_err(|e| format!("invalid G value: {e}"))?;
            let b: f32 = parts[2]
                .parse()
                .map_err(|e| format!("invalid B value: {e}"))?;
            data.push([r, g, b]);
        }
    }

    let size = size.ok_or("missing LUT_3D_SIZE declaration")?;
    let expected = (size as usize).pow(3);

    if data.len() != expected {
        return Err(format!(
            "expected {expected} entries for size {size}, got {}",
            data.len()
        ));
    }

    Ok(Lut3D { size, data })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_cube() {
        let content = "\
# Comment line
TITLE \"Test LUT\"
LUT_3D_SIZE 2

0.0 0.0 0.0
1.0 0.0 0.0
0.0 1.0 0.0
1.0 1.0 0.0
0.0 0.0 1.0
1.0 0.0 1.0
0.0 1.0 1.0
1.0 1.0 1.0
";
        let lut = parse_cube(content).unwrap();
        assert_eq!(lut.size, 2);
        assert_eq!(lut.data.len(), 8); // 2^3
        assert_eq!(lut.data[0], [0.0, 0.0, 0.0]);
        assert_eq!(lut.data[7], [1.0, 1.0, 1.0]);
    }

    #[test]
    fn parse_identity_cube_size3() {
        // Build a size-3 identity LUT: output = input
        let mut lines = String::from("LUT_3D_SIZE 3\n");
        for b in 0..3 {
            for g in 0..3 {
                for r in 0..3 {
                    let rv = r as f32 / 2.0;
                    let gv = g as f32 / 2.0;
                    let bv = b as f32 / 2.0;
                    lines.push_str(&format!("{rv} {gv} {bv}\n"));
                }
            }
        }
        let lut = parse_cube(&lines).unwrap();
        assert_eq!(lut.size, 3);
        assert_eq!(lut.data.len(), 27); // 3^3
    }

    #[test]
    fn parse_with_domain_lines() {
        let content = "\
LUT_3D_SIZE 2
DOMAIN_MIN 0.0 0.0 0.0
DOMAIN_MAX 1.0 1.0 1.0
0.0 0.0 0.0
1.0 0.0 0.0
0.0 1.0 0.0
1.0 1.0 0.0
0.0 0.0 1.0
1.0 0.0 1.0
0.0 1.0 1.0
1.0 1.0 1.0
";
        let lut = parse_cube(content).unwrap();
        assert_eq!(lut.size, 2);
        assert_eq!(lut.data.len(), 8);
    }

    #[test]
    fn parse_missing_size() {
        let content = "0.0 0.0 0.0\n";
        let err = parse_cube(content).unwrap_err();
        assert!(err.contains("missing LUT_3D_SIZE"));
    }

    #[test]
    fn parse_wrong_entry_count() {
        let content = "\
LUT_3D_SIZE 2
0.0 0.0 0.0
1.0 0.0 0.0
";
        let err = parse_cube(content).unwrap_err();
        assert!(err.contains("expected 8 entries"));
    }

    #[test]
    fn parse_size_out_of_range() {
        let content = "LUT_3D_SIZE 0\n";
        let err = parse_cube(content).unwrap_err();
        assert!(err.contains("out of range"));
    }

    #[test]
    fn parse_size_too_large() {
        let content = "LUT_3D_SIZE 300\n";
        let err = parse_cube(content).unwrap_err();
        assert!(err.contains("out of range"));
    }

    #[test]
    fn parse_1d_lut_rejected() {
        let content = "LUT_1D_SIZE 16\n";
        let err = parse_cube(content).unwrap_err();
        assert!(err.contains("1D LUTs are not supported"));
    }

    #[test]
    fn parse_invalid_float() {
        let content = "\
LUT_3D_SIZE 2
0.0 0.0 0.0
abc 0.0 0.0
0.0 0.0 0.0
0.0 0.0 0.0
0.0 0.0 0.0
0.0 0.0 0.0
0.0 0.0 0.0
0.0 0.0 0.0
";
        let err = parse_cube(content).unwrap_err();
        assert!(err.contains("invalid R value"));
    }

    #[test]
    fn parse_comments_and_blank_lines() {
        let content = "\
# This is a comment
# Another comment

TITLE \"My Color LUT\"

LUT_3D_SIZE 2

# Data starts here
0.0 0.0 0.0
1.0 0.0 0.0

0.0 1.0 0.0
1.0 1.0 0.0

0.0 0.0 1.0
1.0 0.0 1.0
0.0 1.0 1.0
1.0 1.0 1.0
";
        let lut = parse_cube(content).unwrap();
        assert_eq!(lut.size, 2);
        assert_eq!(lut.data.len(), 8);
    }

    #[test]
    fn parse_data_values_preserved() {
        let content = "\
LUT_3D_SIZE 2
0.1 0.2 0.3
0.4 0.5 0.6
0.7 0.8 0.9
0.15 0.25 0.35
0.45 0.55 0.65
0.75 0.85 0.95
0.11 0.22 0.33
0.44 0.55 0.66
";
        let lut = parse_cube(content).unwrap();
        assert_eq!(lut.data[0], [0.1, 0.2, 0.3]);
        assert_eq!(lut.data[1], [0.4, 0.5, 0.6]);
        assert_eq!(lut.data[7], [0.44, 0.55, 0.66]);
    }

    #[test]
    fn lut3d_clone() {
        let lut = Lut3D {
            size: 2,
            data: vec![[0.0, 0.0, 0.0]; 8],
        };
        let cloned = lut.clone();
        assert_eq!(cloned.size, lut.size);
        assert_eq!(cloned.data.len(), lut.data.len());
    }
}
