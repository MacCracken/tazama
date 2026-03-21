/// Parsed 3D LUT data.
///
/// Delegates to ranga's `Lut3d` for `.cube` file parsing.
#[derive(Debug, Clone)]
pub struct Lut3D {
    pub size: u32,
    pub data: Vec<[f32; 3]>,
}

/// Parse a `.cube` 3D LUT file via ranga.
///
/// Supports the standard `.cube` format:
/// - `LUT_3D_SIZE N` declares the grid size
/// - Lines starting with `#` are comments
/// - `TITLE` lines are ignored
/// - `DOMAIN_MIN` and `DOMAIN_MAX` lines are ignored (assumed 0..1)
/// - Data lines are `R G B` float triplets, one per line
/// - Total entries must equal `size^3`
pub fn parse_cube(content: &str) -> Result<Lut3D, String> {
    let ranga_lut =
        ranga::filter::Lut3d::from_cube(content).map_err(|e| e.to_string())?;
    Ok(Lut3D {
        size: ranga_lut.size as u32,
        data: ranga_lut.data,
    })
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
        assert_eq!(lut.data.len(), 8);
        assert_eq!(lut.data[0], [0.0, 0.0, 0.0]);
        assert_eq!(lut.data[7], [1.0, 1.0, 1.0]);
    }

    #[test]
    fn parse_identity_cube_size3() {
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
        assert_eq!(lut.data.len(), 27);
    }

    #[test]
    fn parse_missing_size() {
        let content = "0.0 0.0 0.0\n";
        let err = parse_cube(content).unwrap_err();
        assert!(err.contains("LUT_3D_SIZE"));
    }

    #[test]
    fn parse_wrong_entry_count() {
        let content = "\
LUT_3D_SIZE 2
0.0 0.0 0.0
1.0 0.0 0.0
";
        let err = parse_cube(content).unwrap_err();
        assert!(err.contains("expected"));
    }

    #[test]
    fn parse_1d_lut_rejected() {
        let content = "LUT_1D_SIZE 16\n";
        let err = parse_cube(content).unwrap_err();
        assert!(err.contains("1D"));
    }

    #[test]
    fn lut3d_index_pattern() {
        let mut lines = String::from("LUT_3D_SIZE 2\n");
        for b in 0..2u32 {
            for g in 0..2u32 {
                for r in 0..2u32 {
                    lines.push_str(&format!("{} {} {}\n", r as f32, g as f32, b as f32));
                }
            }
        }
        let lut = parse_cube(&lines).unwrap();
        let size = lut.size as usize;
        for b in 0..2usize {
            for g in 0..2usize {
                for r in 0..2usize {
                    let idx = r + g * size + b * size * size;
                    assert_eq!(lut.data[idx], [r as f32, g as f32, b as f32]);
                }
            }
        }
    }
}
