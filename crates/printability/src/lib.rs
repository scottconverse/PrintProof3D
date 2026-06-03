// PrintProof3D Printability Engine
pub mod compatibility;
use printproof3d_core::{
    BoundingBox, BuildVolume, IssueLocation, IssueSeverity, LocationGeometry, MaterialProfile,
    ModelMetadata, PrinterProfile, Triangle, ValidationIssue, ValidationReport, ValidationStatus,
};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub trait ModelValidator {
    fn validate_mesh(
        &self,
        file_path: &Path,
        printer: &PrinterProfile,
        material: &MaterialProfile,
    ) -> Result<ValidationReport, String>;
}

pub trait GcodeValidator {
    fn validate_gcode(
        &self,
        file_path: &Path,
        printer: &PrinterProfile,
        material: &MaterialProfile,
    ) -> Result<ValidationReport, String>;
}

#[derive(Debug, Clone)]
pub struct StlFacet {
    pub normal: [f32; 3],
    pub vertices: [[f32; 3]; 3],
}

fn vertex_key(v: [f32; 3]) -> [i32; 3] {
    [
        (v[0] * 1000.0).round() as i32,
        (v[1] * 1000.0).round() as i32,
        (v[2] * 1000.0).round() as i32,
    ]
}

fn canonical_edge(a: [i32; 3], b: [i32; 3]) -> ([i32; 3], [i32; 3]) {
    if a < b {
        (a, b)
    } else {
        (b, a)
    }
}

fn get_open_edges(facets: &[StlFacet]) -> Vec<([i32; 3], [i32; 3])> {
    let mut edge_counts = HashMap::new();
    for facet in facets {
        let k0 = vertex_key(facet.vertices[0]);
        let k1 = vertex_key(facet.vertices[1]);
        let k2 = vertex_key(facet.vertices[2]);

        // Skip degenerate triangles
        if k0 == k1 || k1 == k2 || k2 == k0 {
            continue;
        }

        let edges = [
            canonical_edge(k0, k1),
            canonical_edge(k1, k2),
            canonical_edge(k2, k0),
        ];
        for edge in edges {
            *edge_counts.entry(edge).or_insert(0) += 1;
        }
    }

    edge_counts
        .into_iter()
        .filter(|(_, count)| *count != 2)
        .map(|(edge, _)| edge)
        .collect()
}

fn magnitude(u: [f32; 3]) -> f32 {
    (u[0] * u[0] + u[1] * u[1] + u[2] * u[2]).sqrt()
}

fn cross_product(u: [f32; 3], v: [f32; 3]) -> [f32; 3] {
    [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ]
}

/// Tolerance (mm) for build-volume bounds checks. A model resting on the bed (Z min ~= 0), placed
/// flush in a corner, or exactly filling the volume must not be failed by sub-millimeter float /
/// placement noise. Matches the tolerance the dedicated below-bed check already uses.
const BUILD_VOLUME_TOL: f32 = 0.05;

/// The facet's effective surface normal for geometry checks (overhang / bridge / bed contact).
/// Prefers the STL file's stored normal when it is usable, but falls back to the geometric normal
/// computed from the vertex winding when the stored normal is missing or degenerate (zero-length).
/// Many exporters — and most binary STLs — write `0 0 0` normals and leave the slicer to derive
/// them from winding; trusting the stored normal there silently disables overhang / bridge / bed
/// detection (the report flips to a false "pass"). Returns `[0,0,0]` only for a genuinely
/// degenerate (zero-area) triangle, which the callers already skip.
fn effective_facet_normal(facet: &StlFacet) -> [f32; 3] {
    let stored = facet.normal;
    let stored_len = magnitude(stored);
    if stored_len >= 1e-6 {
        return [stored[0] / stored_len, stored[1] / stored_len, stored[2] / stored_len];
    }
    let u = [
        facet.vertices[1][0] - facet.vertices[0][0],
        facet.vertices[1][1] - facet.vertices[0][1],
        facet.vertices[1][2] - facet.vertices[0][2],
    ];
    let v = [
        facet.vertices[2][0] - facet.vertices[0][0],
        facet.vertices[2][1] - facet.vertices[0][1],
        facet.vertices[2][2] - facet.vertices[0][2],
    ];
    let n = cross_product(u, v);
    let len = magnitude(n);
    if len < 1e-6 {
        return [0.0, 0.0, 0.0];
    }
    [n[0] / len, n[1] / len, n[2] / len]
}

fn parse_binary_stl(bytes: &[u8]) -> Result<Vec<StlFacet>, String> {
    if bytes.len() < 84 {
        return Err("Binary STL too short".to_string());
    }
    let face_count = u32::from_le_bytes([bytes[80], bytes[81], bytes[82], bytes[83]]) as usize;
    let mut facets = Vec::with_capacity(face_count);
    let mut offset = 84;
    for i in 0..face_count {
        if offset + 50 > bytes.len() {
            return Err(format!("Binary STL truncated at facet {}", i));
        }
        let normal = [
            f32::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ]),
            f32::from_le_bytes([
                bytes[offset + 4],
                bytes[offset + 5],
                bytes[offset + 6],
                bytes[offset + 7],
            ]),
            f32::from_le_bytes([
                bytes[offset + 8],
                bytes[offset + 9],
                bytes[offset + 10],
                bytes[offset + 11],
            ]),
        ];
        offset += 12;
        let v0 = [
            f32::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ]),
            f32::from_le_bytes([
                bytes[offset + 4],
                bytes[offset + 5],
                bytes[offset + 6],
                bytes[offset + 7],
            ]),
            f32::from_le_bytes([
                bytes[offset + 8],
                bytes[offset + 9],
                bytes[offset + 10],
                bytes[offset + 11],
            ]),
        ];
        offset += 12;
        let v1 = [
            f32::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ]),
            f32::from_le_bytes([
                bytes[offset + 4],
                bytes[offset + 5],
                bytes[offset + 6],
                bytes[offset + 7],
            ]),
            f32::from_le_bytes([
                bytes[offset + 8],
                bytes[offset + 9],
                bytes[offset + 10],
                bytes[offset + 11],
            ]),
        ];
        offset += 12;
        let v2 = [
            f32::from_le_bytes([
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ]),
            f32::from_le_bytes([
                bytes[offset + 4],
                bytes[offset + 5],
                bytes[offset + 6],
                bytes[offset + 7],
            ]),
            f32::from_le_bytes([
                bytes[offset + 8],
                bytes[offset + 9],
                bytes[offset + 10],
                bytes[offset + 11],
            ]),
        ];
        offset += 12;
        offset += 2; // skip attributes

        facets.push(StlFacet {
            normal,
            vertices: [v0, v1, v2],
        });
    }
    Ok(facets)
}

fn parse_stl(file_path: &Path) -> Result<Vec<StlFacet>, String> {
    let bytes = std::fs::read(file_path)
        .map_err(|e| format!("Failed to read file {:?}: {}", file_path, e))?;
    if bytes.len() >= 84 {
        let face_count = u32::from_le_bytes([bytes[80], bytes[81], bytes[82], bytes[83]]) as usize;
        if bytes.len() >= 84 + face_count * 50 {
            if let Ok(facets) = parse_binary_stl(&bytes) {
                return Ok(facets);
            }
        }
    }
    parse_ascii_stl(file_path)
}

fn parse_ascii_stl(file_path: &Path) -> Result<Vec<StlFacet>, String> {
    let file =
        File::open(file_path).map_err(|e| format!("Failed to open file {:?}: {}", file_path, e))?;
    let reader = BufReader::new(file);
    let mut facets = Vec::new();
    let mut current_normal = [0.0f32; 3];
    let mut current_vertices = Vec::new();
    let mut in_facet = false;
    let mut in_loop = false;

    for line_opt in reader.lines() {
        let line = line_opt.map_err(|e| format!("Error reading line: {}", e))?;
        let trimmed = line.trim();
        let lower = trimmed.to_lowercase();
        if lower.starts_with("facet normal") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 5 {
                let nx = parts[2]
                    .parse::<f32>()
                    .map_err(|e| format!("Failed to parse normal x: {}", e))?;
                let ny = parts[3]
                    .parse::<f32>()
                    .map_err(|e| format!("Failed to parse normal y: {}", e))?;
                let nz = parts[4]
                    .parse::<f32>()
                    .map_err(|e| format!("Failed to parse normal z: {}", e))?;
                current_normal = [nx, ny, nz];
                in_facet = true;
            }
        } else if lower.starts_with("outer loop") {
            in_loop = true;
            current_vertices.clear();
        } else if lower.starts_with("vertex") {
            if in_loop {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 4 {
                    let vx = parts[1]
                        .parse::<f32>()
                        .map_err(|e| format!("Failed to parse vertex x: {}", e))?;
                    let vy = parts[2]
                        .parse::<f32>()
                        .map_err(|e| format!("Failed to parse vertex y: {}", e))?;
                    let vz = parts[3]
                        .parse::<f32>()
                        .map_err(|e| format!("Failed to parse vertex z: {}", e))?;
                    current_vertices.push([vx, vy, vz]);
                }
            }
        } else if lower.starts_with("endloop") {
            in_loop = false;
        } else if lower.starts_with("endfacet") {
            if in_facet && current_vertices.len() == 3 {
                facets.push(StlFacet {
                    normal: current_normal,
                    vertices: [
                        current_vertices[0],
                        current_vertices[1],
                        current_vertices[2],
                    ],
                });
            }
            in_facet = false;
            current_vertices.clear();
        }
    }
    Ok(facets)
}

pub struct StlModelValidator;

impl ModelValidator for StlModelValidator {
    fn validate_mesh(
        &self,
        file_path: &Path,
        printer: &PrinterProfile,
        material: &MaterialProfile,
    ) -> Result<ValidationReport, String> {
        let file_name = file_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unknown".to_string());

        let facets = parse_stl(file_path)?;
        if facets.is_empty() {
            return Err("STL file contains no facets or is not a valid ASCII STL file".to_string());
        }

        let mut issues = Vec::new();

        // 1. Manifold / Watertight Check
        let open_edges = get_open_edges(&facets);
        if !open_edges.is_empty() {
            issues.push(ValidationIssue {
                id: "MESH_NOT_MANIFOLD".to_string(),
                severity: IssueSeverity::Critical,
                message: format!(
                    "Model mesh is not watertight/manifold. Found {} open/non-manifold edges.",
                    open_edges.len()
                ),
                location: Some(IssueLocation {
                    region: "mesh_boundaries".to_string(),
                    geometry: None,
                }),
                suggested_fixes: vec![
                    "Repair the 3D model in a mesh editor (e.g. Blender, Netfabb) to make it watertight.".to_string(),
                ],
            });
        }

        // Calculate bounding box bounds
        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        let mut min_z = f32::MAX;
        let mut max_z = f32::MIN;

        for facet in &facets {
            for v in &facet.vertices {
                if v[0] < min_x {
                    min_x = v[0];
                }
                if v[0] > max_x {
                    max_x = v[0];
                }
                if v[1] < min_y {
                    min_y = v[1];
                }
                if v[1] > max_y {
                    max_y = v[1];
                }
                if v[2] < min_z {
                    min_z = v[2];
                }
                if v[2] > max_z {
                    max_z = v[2];
                }
            }
        }

        // 2. Build Volume Fit Check
        let mut out_of_bounds = false;
        match &printer.build_volume {
            BuildVolume::Rectangular { x, y, z } => {
                if min_x < -BUILD_VOLUME_TOL
                    || max_x > *x + BUILD_VOLUME_TOL
                    || min_y < -BUILD_VOLUME_TOL
                    || max_y > *y + BUILD_VOLUME_TOL
                    || min_z < -BUILD_VOLUME_TOL
                    || max_z > *z + BUILD_VOLUME_TOL
                {
                    out_of_bounds = true;
                }
            }
            BuildVolume::Cylindrical { diameter, z } => {
                let r_max = diameter / 2.0 + BUILD_VOLUME_TOL;
                for facet in &facets {
                    for v in &facet.vertices {
                        let r2 = v[0] * v[0] + v[1] * v[1];
                        if r2 > r_max * r_max
                            || v[2] < -BUILD_VOLUME_TOL
                            || v[2] > *z + BUILD_VOLUME_TOL
                        {
                            out_of_bounds = true;
                            break;
                        }
                    }
                    if out_of_bounds {
                        break;
                    }
                }
            }
        }

        if out_of_bounds {
            issues.push(ValidationIssue {
                id: "MODEL_OUT_OF_BOUNDS".to_string(),
                severity: IssueSeverity::Critical,
                message: format!(
                    "Model dimensions [X: {:.2}..{:.2}, Y: {:.2}..{:.2}, Z: {:.2}..{:.2}] exceed printer build volume limits.",
                    min_x, max_x, min_y, max_y, min_z, max_z
                ),
                location: Some(IssueLocation {
                    region: "outer_bounds".to_string(),
                    geometry: Some(LocationGeometry::BoundingBox(BoundingBox {
                        min_x, min_y, min_z, max_x, max_y, max_z
                    })),
                }),
                suggested_fixes: vec![
                    "Scale down the model to fit the build volume.".to_string(),
                    "Rotate or reposition the model on the build plate.".to_string(),
                ],
            });
        }

        // 2a. Model Oversized Check
        let mut is_oversized = false;
        let (limit_x, limit_y, limit_z) = match &printer.build_volume {
            BuildVolume::Rectangular { x, y, z } => (*x, *y, *z),
            BuildVolume::Cylindrical { diameter, z } => (*diameter, *diameter, *z),
        };
        let model_w = max_x - min_x;
        let model_d = max_y - min_y;
        let model_h = max_z - min_z;
        if model_w > limit_x || model_d > limit_y || model_h > limit_z {
            is_oversized = true;
        }

        if is_oversized {
            issues.push(ValidationIssue {
                id: "MODEL_OVERSIZED".to_string(),
                severity: IssueSeverity::Critical,
                message: format!(
                    "Model bounding box dimensions [W: {:.2}, D: {:.2}, H: {:.2}] mm exceed printer print volume limits [W: {:.2}, D: {:.2}, H: {:.2}] mm.",
                    model_w, model_d, model_h, limit_x, limit_y, limit_z
                ),
                location: Some(IssueLocation {
                    region: "outer_bounds".to_string(),
                    geometry: Some(LocationGeometry::BoundingBox(BoundingBox {
                        min_x, min_y, min_z, max_x, max_y, max_z
                    })),
                }),
                suggested_fixes: vec![
                    "Scale down the model to fit within the build volume.".to_string(),
                ],
            });
        }

        // 2b. Below Bed Geometry Check
        if min_z < -BUILD_VOLUME_TOL {
            issues.push(ValidationIssue {
                id: "BELOW_BED_GEOMETRY".to_string(),
                severity: IssueSeverity::Major,
                message: format!(
                    "Model coordinates extend below the bed surface (Z minimum is {:.2} mm).",
                    min_z
                ),
                location: Some(IssueLocation {
                    region: "outer_bounds".to_string(),
                    geometry: None,
                }),
                suggested_fixes: vec![
                    "Translate the model upward along the Z axis.".to_string(),
                    "Cut off the bottom section of the model in the slicer.".to_string(),
                ],
            });
        }

        // 2c. Degenerate Triangles Check
        let mut degenerate_count = 0;
        for facet in &facets {
            let u = [
                facet.vertices[1][0] - facet.vertices[0][0],
                facet.vertices[1][1] - facet.vertices[0][1],
                facet.vertices[1][2] - facet.vertices[0][2],
            ];
            let v = [
                facet.vertices[2][0] - facet.vertices[0][0],
                facet.vertices[2][1] - facet.vertices[0][1],
                facet.vertices[2][2] - facet.vertices[0][2],
            ];
            let cp = cross_product(u, v);
            let area = 0.5 * magnitude(cp);

            let q0 = [
                (facet.vertices[0][0] * 1000.0).round() as i32,
                (facet.vertices[0][1] * 1000.0).round() as i32,
                (facet.vertices[0][2] * 1000.0).round() as i32,
            ];
            let q1 = [
                (facet.vertices[1][0] * 1000.0).round() as i32,
                (facet.vertices[1][1] * 1000.0).round() as i32,
                (facet.vertices[1][2] * 1000.0).round() as i32,
            ];
            let q2 = [
                (facet.vertices[2][0] * 1000.0).round() as i32,
                (facet.vertices[2][1] * 1000.0).round() as i32,
                (facet.vertices[2][2] * 1000.0).round() as i32,
            ];

            if area < 1e-6 || q0 == q1 || q1 == q2 || q2 == q0 {
                degenerate_count += 1;
            }
        }
        if degenerate_count > 0 {
            issues.push(ValidationIssue {
                id: "DEGENERATE_TRIANGLES".to_string(),
                severity: IssueSeverity::Minor,
                message: format!(
                    "Model mesh contains {} degenerate or zero-area triangles.",
                    degenerate_count
                ),
                location: Some(IssueLocation {
                    region: "mesh_quality".to_string(),
                    geometry: None,
                }),
                suggested_fixes: vec![
                    "Repair degenerate facets in a mesh repair tool before slicing.".to_string(),
                ],
            });
        }

        // 3. Overhang and Bridge Detection
        let mut overhang_triangles = Vec::new();
        let mut bridge_triangles = Vec::new();

        let overhang_thresh_deg = match material.overhang_difficulty {
            printproof3d_core::RiskLevel::Low => 45.0f32,
            printproof3d_core::RiskLevel::Medium => 50.0,
            printproof3d_core::RiskLevel::High => 55.0,
        };
        let overhang_cos_thresh = (overhang_thresh_deg.to_radians()).cos();

        for facet in &facets {
            // Use the geometric normal when the file normal is missing/degenerate, so a mesh with
            // zeroed STL normals doesn't silently skip overhang/bridge detection.
            let n = effective_facet_normal(facet);
            let len = magnitude(n);
            if len < 1e-6 {
                continue;
            }

            // Facing downwards
            if n[2] < -0.01 {
                let min_facet_z = facet.vertices.iter().map(|v| v[2]).fold(f32::MAX, f32::min);
                if min_facet_z > BUILD_VOLUME_TOL {
                    let cos_theta = -n[2] / len;
                    if cos_theta > 0.99 {
                        bridge_triangles.push(Triangle {
                            v0: facet.vertices[0],
                            v1: facet.vertices[1],
                            v2: facet.vertices[2],
                        });
                    } else if cos_theta < overhang_cos_thresh {
                        overhang_triangles.push(Triangle {
                            v0: facet.vertices[0],
                            v1: facet.vertices[1],
                            v2: facet.vertices[2],
                        });
                    }
                }
            }
        }

        if !overhang_triangles.is_empty() {
            issues.push(ValidationIssue {
                id: "OVERHANG_UNSUPPORTED".to_string(),
                severity: IssueSeverity::Major,
                message: format!(
                    "Found {} steep unsupported overhang triangles exceeding material limits ({:.1}°).",
                    overhang_triangles.len(), overhang_thresh_deg
                ),
                location: Some(IssueLocation {
                    region: "overhangs".to_string(),
                    geometry: Some(LocationGeometry::Triangles { triangles: overhang_triangles }),
                }),
                suggested_fixes: vec![
                    "Add support structures in your slicer software.".to_string(),
                    "Reorient the model to reduce overhang angles.".to_string(),
                ],
            });
        }

        if !bridge_triangles.is_empty() {
            issues.push(ValidationIssue {
                id: "BRIDGE_UNSUPPORTED".to_string(),
                severity: IssueSeverity::Minor,
                message: format!(
                    "Found {} horizontal bridge/ceiling triangles suspended in the air.",
                    bridge_triangles.len()
                ),
                location: Some(IssueLocation {
                    region: "bridges".to_string(),
                    geometry: Some(LocationGeometry::Triangles {
                        triangles: bridge_triangles,
                    }),
                }),
                suggested_fixes: vec![
                    "Enable bridging settings in the slicer.".to_string(),
                    "Add supports if the bridge spans a long distance.".to_string(),
                ],
            });
        }

        // 4. Bed Adhesion Heuristics
        let mut bed_contact_area = 0.0f32;
        for facet in &facets {
            let on_bed = facet.vertices.iter().all(|v| v[2] < BUILD_VOLUME_TOL);
            let facing_down = effective_facet_normal(facet)[2] < -0.9;
            if on_bed && facing_down {
                let u = [
                    facet.vertices[1][0] - facet.vertices[0][0],
                    facet.vertices[1][1] - facet.vertices[0][1],
                    facet.vertices[1][2] - facet.vertices[0][2],
                ];
                let v = [
                    facet.vertices[2][0] - facet.vertices[0][0],
                    facet.vertices[2][1] - facet.vertices[0][1],
                    facet.vertices[2][2] - facet.vertices[0][2],
                ];
                let cp = cross_product(u, v);
                let area = 0.5 * magnitude(cp);
                bed_contact_area += area;
            }
        }

        let footprint_area = (max_x - min_x) * (max_y - min_y);
        if footprint_area > 0.0 {
            let ratio = bed_contact_area / footprint_area;
            if ratio < 0.05 || bed_contact_area < 10.0 {
                let severity = match material.warp_risk {
                    printproof3d_core::RiskLevel::High => IssueSeverity::Major,
                    _ => IssueSeverity::Minor,
                };
                issues.push(ValidationIssue {
                    id: "POOR_BED_ADHESION".to_string(),
                    severity,
                    message: format!(
                        "Low bed contact area ({:.2} mm², {:.1}% of footprint). High risk of model detaching during print.",
                        bed_contact_area, ratio * 100.0
                    ),
                    location: Some(IssueLocation {
                        region: "bed_contact".to_string(),
                        geometry: None,
                    }),
                    suggested_fixes: vec![
                        "Add a brim or raft around the model base.".to_string(),
                        "Use bed adhesive (glue stick, hairspray) to improve stickiness.".to_string(),
                    ],
                });
            }
        }

        // Determine ValidationStatus
        let mut status = ValidationStatus::Pass;
        for issue in &issues {
            match issue.severity {
                IssueSeverity::Blocker | IssueSeverity::Critical => {
                    status = ValidationStatus::Fail;
                    break;
                }
                IssueSeverity::Major if status != ValidationStatus::Fail => {
                    status = ValidationStatus::Warning;
                }
                _ => {}
            }
        }

        let model_bb = BoundingBox {
            min_x,
            min_y,
            min_z,
            max_x,
            max_y,
            max_z,
        };

        Ok(ValidationReport {
            status,
            target_printer_profile: format!("{}_{}", printer.manufacturer, printer.model),
            target_material_profile: material.name.clone(),
            model: ModelMetadata {
                file_name,
                units: "mm".to_string(),
                bounding_box: model_bb,
            },
            issues,
            confidence_level: if open_edges.is_empty() {
                "high".to_string()
            } else {
                "medium".to_string()
            },
            sliced_settings_assumed: None,
        })
    }
}

pub struct StandardGcodeValidator;

fn get_gcode_param(words: &[&str], prefix: char) -> Option<f32> {
    let lower_prefix = prefix.to_ascii_lowercase();
    let upper_prefix = prefix.to_ascii_uppercase();
    for word in words {
        if word.starts_with(lower_prefix) || word.starts_with(upper_prefix) {
            if let Ok(val) = word[1..].parse::<f32>() {
                return Some(val);
            }
        }
    }
    None
}

#[allow(clippy::too_many_arguments)]
fn update_bbox(
    x: f32,
    y: f32,
    z: f32,
    min_x: &mut f32,
    max_x: &mut f32,
    min_y: &mut f32,
    max_y: &mut f32,
    min_z: &mut f32,
    max_z: &mut f32,
) {
    if x < *min_x {
        *min_x = x;
    }
    if x > *max_x {
        *max_x = x;
    }
    if y < *min_y {
        *min_y = y;
    }
    if y > *max_y {
        *max_y = y;
    }
    if z < *min_z {
        *min_z = z;
    }
    if z > *max_z {
        *max_z = z;
    }
}

impl GcodeValidator for StandardGcodeValidator {
    fn validate_gcode(
        &self,
        file_path: &Path,
        printer: &PrinterProfile,
        material: &MaterialProfile,
    ) -> Result<ValidationReport, String> {
        let file_name = file_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unknown".to_string());

        let file = File::open(file_path)
            .map_err(|e| format!("Failed to open file {:?}: {}", file_path, e))?;
        let reader = BufReader::new(file);

        let mut current_x = 0.0f32;
        let mut current_y = 0.0f32;
        let mut current_z = 0.0f32;
        let mut current_e = 0.0f32;
        let mut absolute_xyz = true;
        let mut absolute_e = true;

        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        let mut min_z = f32::MAX;
        let mut max_z = f32::MIN;

        let mut issues = Vec::new();

        // 1. Unsupported File Type Check
        let ext = file_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();

        let is_supported = printer
            .supported_file_types
            .iter()
            .any(|t| t.to_lowercase() == ext);

        if !is_supported {
            issues.push(ValidationIssue {
                id: "UNSUPPORTED_FILE_TYPE".to_string(),
                severity: IssueSeverity::Critical,
                message: format!(
                    "File type extension '{}' is not supported by target printer profile. Supported: {:?}.",
                    ext, printer.supported_file_types
                ),
                location: Some(IssueLocation {
                    region: "file_format".to_string(),
                    geometry: None,
                }),
                suggested_fixes: vec![
                    "Slice the model using a file format supported by your printer profile.".to_string(),
                ],
            });
        }

        let mut line_number = 0;
        let mut homed = false;
        let mut current_nozzle_temp = 0.0f32;

        let mut alert_gcode_out_of_bounds = false;
        let mut alert_hotend_temp_exceeds_max = false;
        let mut alert_hotend_temp_out_of_range = false;
        let mut alert_bed_temp_exceeds_max = false;
        let mut alert_bed_temp_out_of_range = false;
        let mut alert_cold_extrusion = false;
        let mut alert_missing_homing = false;
        let mut alert_unsafe_command = false;

        for line_opt in reader.lines() {
            line_number += 1;
            let line =
                line_opt.map_err(|e| format!("Error reading line {}: {}", line_number, e))?;

            let gcode_part = match line.split(';').next() {
                Some(s) => s.trim(),
                None => continue,
            };
            if gcode_part.is_empty() {
                continue;
            }

            let words: Vec<&str> = gcode_part.split_whitespace().collect();
            if words.is_empty() {
                continue;
            }

            let cmd = words[0].to_uppercase();

            // 2. Unsafe Command Check
            if printer
                .unsafe_commands
                .iter()
                .any(|c| c.to_uppercase() == cmd)
                && !alert_unsafe_command
            {
                alert_unsafe_command = true;
                issues.push(ValidationIssue {
                    id: "UNSAFE_COMMAND_BLOCKED".to_string(),
                    severity: IssueSeverity::Critical,
                    message: format!(
                        "Unsafe G-code command '{}' detected at line {} is blocked by printer profile.",
                        cmd, line_number
                    ),
                    location: Some(IssueLocation {
                        region: "security_limits".to_string(),
                        geometry: None,
                    }),
                    suggested_fixes: vec![
                        "Remove the unsafe G-code command from custom script settings in your slicer.".to_string(),
                    ],
                });
            }

            match cmd.as_str() {
                "G90" => {
                    absolute_xyz = true;
                }
                "G91" => {
                    absolute_xyz = false;
                }
                "M82" => {
                    absolute_e = true;
                }
                "M83" => {
                    absolute_e = false;
                }
                "G92" => {
                    if let Some(x) = get_gcode_param(&words, 'X') {
                        current_x = x;
                    }
                    if let Some(y) = get_gcode_param(&words, 'Y') {
                        current_y = y;
                    }
                    if let Some(z) = get_gcode_param(&words, 'Z') {
                        current_z = z;
                    }
                    if let Some(e) = get_gcode_param(&words, 'E') {
                        current_e = e;
                    }
                }
                "G28" => {
                    homed = true;
                    let has_x = get_gcode_param(&words, 'X').is_some();
                    let has_y = get_gcode_param(&words, 'Y').is_some();
                    let has_z = get_gcode_param(&words, 'Z').is_some();
                    let home_all = !has_x && !has_y && !has_z;

                    if home_all || has_x {
                        current_x = 0.0;
                    }
                    if home_all || has_y {
                        current_y = 0.0;
                    }
                    if home_all || has_z {
                        current_z = 0.0;
                    }

                    update_bbox(
                        current_x, current_y, current_z, &mut min_x, &mut max_x, &mut min_y,
                        &mut max_y, &mut min_z, &mut max_z,
                    );
                }
                "G0" | "G1" | "G2" | "G3" => {
                    // 3. Missing Homing Check
                    if !homed && !alert_missing_homing {
                        alert_missing_homing = true;
                        issues.push(ValidationIssue {
                            id: "MISSING_HOMING".to_string(),
                            severity: IssueSeverity::Major,
                            message: format!(
                                "Movement command '{}' detected at line {} before homing instruction (G28).",
                                cmd, line_number
                            ),
                            location: Some(IssueLocation {
                                region: "motion_limits".to_string(),
                                geometry: None,
                            }),
                            suggested_fixes: vec![
                                "Add a G28 homing command to your start G-code in the slicer.".to_string(),
                            ],
                        });
                    }

                    // 4. Cold Extrusion Check
                    let de = get_gcode_param(&words, 'E');
                    if let Some(e) = de {
                        let is_extruding = if absolute_e { e > current_e } else { e > 0.0 };

                        if is_extruding && !alert_cold_extrusion {
                            let min_temp = if material.name != "Generic" {
                                material.min_nozzle_temp.max(170.0)
                            } else {
                                170.0
                            };
                            if current_nozzle_temp < min_temp {
                                alert_cold_extrusion = true;
                                issues.push(ValidationIssue {
                                    id: "COLD_EXTRUSION".to_string(),
                                    severity: IssueSeverity::Major,
                                    message: format!(
                                        "Extrusion command at line {} attempts extrusion (E={:.4}) at cold nozzle temperature of {:.1}°C (minimum is {:.1}°C).",
                                        line_number, e, current_nozzle_temp, min_temp
                                    ),
                                    location: Some(IssueLocation {
                                        region: "thermal_limits".to_string(),
                                        geometry: None,
                                    }),
                                    suggested_fixes: vec![
                                        "Ensure the extruder is heated (using M109 or M104) before extrusion movements.".to_string(),
                                    ],
                                });
                            }
                        }

                        if absolute_e {
                            current_e = e;
                        } else {
                            current_e += e;
                        }
                    }

                    let dx = get_gcode_param(&words, 'X');
                    let dy = get_gcode_param(&words, 'Y');
                    let dz = get_gcode_param(&words, 'Z');

                    if absolute_xyz {
                        if let Some(x) = dx {
                            current_x = x;
                        }
                        if let Some(y) = dy {
                            current_y = y;
                        }
                        if let Some(z) = dz {
                            current_z = z;
                        }
                    } else {
                        if let Some(x) = dx {
                            current_x += x;
                        }
                        if let Some(y) = dy {
                            current_y += y;
                        }
                        if let Some(z) = dz {
                            current_z += z;
                        }
                    }

                    update_bbox(
                        current_x, current_y, current_z, &mut min_x, &mut max_x, &mut min_y,
                        &mut max_y, &mut min_z, &mut max_z,
                    );

                    let mut out_of_bounds = false;
                    match &printer.build_volume {
                        BuildVolume::Rectangular { x, y, z } => {
                            if current_x < 0.0
                                || current_x > *x
                                || current_y < 0.0
                                || current_y > *y
                                || current_z < 0.0
                                || current_z > *z
                            {
                                out_of_bounds = true;
                            }
                        }
                        BuildVolume::Cylindrical { diameter, z } => {
                            let r_max = diameter / 2.0;
                            let r2 = current_x * current_x + current_y * current_y;
                            if r2 > r_max * r_max || current_z < 0.0 || current_z > *z {
                                out_of_bounds = true;
                            }
                        }
                    }

                    if out_of_bounds && !alert_gcode_out_of_bounds {
                        alert_gcode_out_of_bounds = true;
                        issues.push(ValidationIssue {
                            id: "GCODE_OUT_OF_BOUNDS".to_string(),
                            severity: IssueSeverity::Critical,
                            message: format!(
                                "G-code move at line {} attempts to position outside build volume bounds at [X: {:.2}, Y: {:.2}, Z: {:.2}].",
                                line_number, current_x, current_y, current_z
                            ),
                            location: Some(IssueLocation {
                                region: "motion_limits".to_string(),
                                geometry: Some(LocationGeometry::Point {
                                    x: current_x,
                                    y: current_y,
                                    z: current_z,
                                }),
                            }),
                            suggested_fixes: vec![
                                "Verify that your slicer bed dimensions match your printer profile.".to_string(),
                                "Center the model on the build plate in the slicer.".to_string(),
                            ],
                        });
                    }
                }
                "M104" | "M109" => {
                    if let Some(temp) = get_gcode_param(&words, 'S') {
                        current_nozzle_temp = temp;
                        if temp > printer.max_hotend_temp && !alert_hotend_temp_exceeds_max {
                            alert_hotend_temp_exceeds_max = true;
                            issues.push(ValidationIssue {
                                id: "HOTEND_TEMP_EXCEEDS_MAX".to_string(),
                                severity: IssueSeverity::Critical,
                                message: format!(
                                    "Target hotend temperature of {:.1}°C at line {} exceeds printer limit of {:.1}°C.",
                                    temp, line_number, printer.max_hotend_temp
                                ),
                                location: Some(IssueLocation {
                                    region: "thermal_limits".to_string(),
                                    geometry: None,
                                }),
                                suggested_fixes: vec![
                                    "Reduce the extrusion temperature in the slicer filament settings.".to_string(),
                                ],
                            });
                        }
                        if material.name != "Generic"
                            && temp > 0.0
                            && (temp < material.min_nozzle_temp || temp > material.max_nozzle_temp)
                            && !alert_hotend_temp_out_of_range
                        {
                            alert_hotend_temp_out_of_range = true;
                            issues.push(ValidationIssue {
                                id: "HOTEND_TEMP_OUT_OF_RANGE".to_string(),
                                severity: IssueSeverity::Major,
                                message: format!(
                                    "Target hotend temperature of {:.1}°C at line {} is outside recommended filament range ({:.1}°C - {:.1}°C).",
                                    temp, line_number, material.min_nozzle_temp, material.max_nozzle_temp
                                ),
                                location: Some(IssueLocation {
                                    region: "thermal_limits".to_string(),
                                    geometry: None,
                                }),
                                suggested_fixes: vec![
                                    "Adjust print temperature to stay within recommended filament bounds.".to_string(),
                                ],
                            });
                        }
                    }
                }
                "M140" | "M190" => {
                    if let Some(temp) = get_gcode_param(&words, 'S') {
                        if temp > printer.max_bed_temp && !alert_bed_temp_exceeds_max {
                            alert_bed_temp_exceeds_max = true;
                            issues.push(ValidationIssue {
                                id: "BED_TEMP_EXCEEDS_MAX".to_string(),
                                severity: IssueSeverity::Critical,
                                message: format!(
                                    "Target bed temperature of {:.1}°C at line {} exceeds printer limit of {:.1}°C.",
                                    temp, line_number, printer.max_bed_temp
                                ),
                                location: Some(IssueLocation {
                                    region: "thermal_limits".to_string(),
                                    geometry: None,
                                }),
                                suggested_fixes: vec![
                                    "Reduce the bed temperature in the slicer filament settings.".to_string(),
                                ],
                            });
                        }
                        if material.name != "Generic"
                            && temp > 0.0
                            && (temp < material.min_bed_temp || temp > material.max_bed_temp)
                            && !alert_bed_temp_out_of_range
                        {
                            alert_bed_temp_out_of_range = true;
                            issues.push(ValidationIssue {
                                id: "BED_TEMP_OUT_OF_RANGE".to_string(),
                                severity: IssueSeverity::Major,
                                message: format!(
                                    "Target bed temperature of {:.1}°C at line {} is outside recommended filament range ({:.1}°C - {:.1}°C).",
                                    temp, line_number, material.min_bed_temp, material.max_bed_temp
                                ),
                                location: Some(IssueLocation {
                                    region: "thermal_limits".to_string(),
                                    geometry: None,
                                }),
                                suggested_fixes: vec![
                                    "Adjust bed temperature to stay within recommended filament bounds.".to_string(),
                                ],
                            });
                        }
                    }
                }
                _ => {}
            }
        }

        if min_x > max_x {
            min_x = 0.0;
            max_x = 0.0;
            min_y = 0.0;
            max_y = 0.0;
            min_z = 0.0;
            max_z = 0.0;
        }

        let model_bb = BoundingBox {
            min_x,
            min_y,
            min_z,
            max_x,
            max_y,
            max_z,
        };

        let mut status = ValidationStatus::Pass;
        for issue in &issues {
            match issue.severity {
                IssueSeverity::Blocker | IssueSeverity::Critical => {
                    status = ValidationStatus::Fail;
                    break;
                }
                IssueSeverity::Major if status != ValidationStatus::Fail => {
                    status = ValidationStatus::Warning;
                }
                _ => {}
            }
        }

        Ok(ValidationReport {
            status,
            target_printer_profile: format!("{}_{}", printer.manufacturer, printer.model),
            target_material_profile: material.name.clone(),
            model: ModelMetadata {
                file_name,
                units: "mm".to_string(),
                bounding_box: model_bb,
            },
            issues,
            confidence_level: if homed {
                "high".to_string()
            } else {
                "low".to_string()
            },
            sliced_settings_assumed: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn get_fixtures_and_profiles() -> (PathBuf, PrinterProfile, MaterialProfile) {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let fixtures_dir = manifest_dir.join("../../fixtures");
        let profiles_dir = manifest_dir.join("../../profiles");

        let printer_json = std::fs::read_to_string(profiles_dir.join("prusa_mk4.json")).unwrap();
        let printer: PrinterProfile = serde_json::from_str(&printer_json).unwrap();

        let material_json = std::fs::read_to_string(profiles_dir.join("pla.json")).unwrap();
        let material: MaterialProfile = serde_json::from_str(&material_json).unwrap();

        (fixtures_dir, printer, material)
    }

    #[test]
    fn test_validate_mesh_tetrahedron() {
        let (fixtures_dir, printer, material) = get_fixtures_and_profiles();
        let path = fixtures_dir.join("tetrahedron.stl");

        let validator = StlModelValidator;
        let report = validator.validate_mesh(&path, &printer, &material).unwrap();

        // Tetrahedron is watertight, so it should not fail on watertight checks.
        // It might trigger poor bed adhesion due to small footprint, but its status should be Pass or Warning, NOT Fail.
        assert_ne!(report.status, ValidationStatus::Fail);
        assert!(report
            .issues
            .iter()
            .all(|issue| issue.id != "MESH_NOT_MANIFOLD"));
    }

    #[test]
    fn test_validate_mesh_open_triangle() {
        let (fixtures_dir, printer, material) = get_fixtures_and_profiles();
        let path = fixtures_dir.join("open_triangle.stl");

        let validator = StlModelValidator;
        let report = validator.validate_mesh(&path, &printer, &material).unwrap();

        // A single facet is open and not manifold
        assert_eq!(report.status, ValidationStatus::Fail);
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.id == "MESH_NOT_MANIFOLD"));
    }

    #[test]
    fn test_validate_mesh_overhang_flange() {
        let (fixtures_dir, printer, material) = get_fixtures_and_profiles();
        let path = fixtures_dir.join("overhang_flange.stl");

        let validator = StlModelValidator;
        let report = validator.validate_mesh(&path, &printer, &material).unwrap();

        // Overhang flange is watertight but has overhangs, status should be Warning
        assert_eq!(report.status, ValidationStatus::Warning);
        assert!(report
            .issues
            .iter()
            .all(|issue| issue.id != "MESH_NOT_MANIFOLD"));
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.id == "OVERHANG_UNSUPPORTED" || issue.id == "BRIDGE_UNSUPPORTED"));
    }

    #[test]
    fn test_validate_gcode_safe() {
        let (fixtures_dir, printer, material) = get_fixtures_and_profiles();
        let path = fixtures_dir.join("safe_print.gcode");

        let validator = StandardGcodeValidator;
        let report = validator
            .validate_gcode(&path, &printer, &material)
            .unwrap();

        assert_eq!(report.status, ValidationStatus::Pass);
        assert!(report.issues.is_empty());
    }

    #[test]
    fn test_validate_gcode_out_of_bounds() {
        let (fixtures_dir, printer, material) = get_fixtures_and_profiles();
        let path = fixtures_dir.join("out_of_bounds.gcode");

        let validator = StandardGcodeValidator;
        let report = validator
            .validate_gcode(&path, &printer, &material)
            .unwrap();

        assert_eq!(report.status, ValidationStatus::Fail);
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.id == "GCODE_OUT_OF_BOUNDS"));
    }

    #[test]
    fn test_validate_gcode_unsafe_temp() {
        let (fixtures_dir, printer, material) = get_fixtures_and_profiles();
        let path = fixtures_dir.join("unsafe_temp.gcode");

        let validator = StandardGcodeValidator;
        let report = validator
            .validate_gcode(&path, &printer, &material)
            .unwrap();

        assert_eq!(report.status, ValidationStatus::Fail);
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.id == "HOTEND_TEMP_EXCEEDS_MAX"));
    }

    #[test]
    fn test_parse_binary_stl() {
        let mut bytes = vec![0u8; 80]; // Header
        bytes.extend_from_slice(&1u32.to_le_bytes()); // 1 Triangle

        // Facet normal: [0.0, 0.0, -1.0]
        bytes.extend_from_slice(&0.0f32.to_le_bytes());
        bytes.extend_from_slice(&0.0f32.to_le_bytes());
        bytes.extend_from_slice(&(-1.0f32).to_le_bytes());

        // Vertex 0: [0.0, 0.0, 10.0]
        bytes.extend_from_slice(&0.0f32.to_le_bytes());
        bytes.extend_from_slice(&0.0f32.to_le_bytes());
        bytes.extend_from_slice(&10.0f32.to_le_bytes());

        // Vertex 1: [10.0, 0.0, 10.0]
        bytes.extend_from_slice(&10.0f32.to_le_bytes());
        bytes.extend_from_slice(&0.0f32.to_le_bytes());
        bytes.extend_from_slice(&10.0f32.to_le_bytes());

        // Vertex 2: [5.0, 5.0, 10.0]
        bytes.extend_from_slice(&5.0f32.to_le_bytes());
        bytes.extend_from_slice(&5.0f32.to_le_bytes());
        bytes.extend_from_slice(&10.0f32.to_le_bytes());

        bytes.extend_from_slice(&0u16.to_le_bytes()); // Attribute byte count

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("temp_test_binary.stl");
        std::fs::write(&file_path, bytes).unwrap();

        let facets = parse_stl(&file_path).unwrap();
        std::fs::remove_file(file_path).unwrap();

        assert_eq!(facets.len(), 1);
        assert_eq!(facets[0].normal, [0.0, 0.0, -1.0]);
        assert_eq!(facets[0].vertices[0], [0.0, 0.0, 10.0]);
        assert_eq!(facets[0].vertices[1], [10.0, 0.0, 10.0]);
        assert_eq!(facets[0].vertices[2], [5.0, 5.0, 10.0]);
    }

    #[test]
    fn test_validate_gcode_deduplication() {
        let (_, printer, material) = get_fixtures_and_profiles();
        // Create a temporary G-code file with multiple duplicate GCODE_OUT_OF_BOUNDS coordinates
        let gcode_content = "G28\nG1 X300 Y300 Z300\nG1 X300 Y300 Z300\nG1 X300 Y300 Z300\n";

        let temp_dir = std::env::temp_dir();
        let file_path = temp_dir.join("temp_test_dedup.gcode");
        std::fs::write(&file_path, gcode_content).unwrap();

        let validator = StandardGcodeValidator;
        let report = validator
            .validate_gcode(&file_path, &printer, &material)
            .unwrap();
        std::fs::remove_file(file_path).unwrap();

        // Check that only 1 issue for GCODE_OUT_OF_BOUNDS is present
        let bounds_issues_count = report
            .issues
            .iter()
            .filter(|issue| issue.id == "GCODE_OUT_OF_BOUNDS")
            .count();
        assert_eq!(bounds_issues_count, 1);
    }

    #[test]
    fn test_validate_mesh_oversized() {
        let (fixtures_dir, printer, material) = get_fixtures_and_profiles();
        let path = fixtures_dir.join("oversized_cube.stl");

        let validator = StlModelValidator;
        let report = validator.validate_mesh(&path, &printer, &material).unwrap();

        assert_eq!(report.status, ValidationStatus::Fail);
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.id == "MODEL_OVERSIZED"));
    }

    #[test]
    fn test_validate_mesh_below_bed() {
        let (fixtures_dir, printer, material) = get_fixtures_and_profiles();
        let path = fixtures_dir.join("below_bed_cube.stl");

        let validator = StlModelValidator;
        let report = validator.validate_mesh(&path, &printer, &material).unwrap();

        assert_eq!(report.status, ValidationStatus::Fail);
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.id == "BELOW_BED_GEOMETRY"));
    }

    #[test]
    fn test_effective_facet_normal_falls_back_to_geometry() {
        // A downward-facing triangle whose stored normal is zeroed: the helper must return the
        // geometric normal (pointing -Z), not a useless [0,0,0].
        let zeroed = StlFacet {
            normal: [0.0, 0.0, 0.0],
            vertices: [[0.0, 0.0, 5.0], [0.0, 10.0, 5.0], [10.0, 0.0, 5.0]],
        };
        let n = effective_facet_normal(&zeroed);
        assert!(magnitude(n) > 0.99, "expected a unit geometric normal, got {:?}", n);
        assert!(n[2] < -0.99, "expected the geometric normal to point down (-Z), got {:?}", n);

        // A facet with a usable stored normal is returned unchanged (no regression for good files).
        let good = StlFacet {
            normal: [0.0, 0.0, 1.0],
            vertices: [[0.0, 0.0, 5.0], [10.0, 0.0, 5.0], [0.0, 10.0, 5.0]],
        };
        assert_eq!(effective_facet_normal(&good), [0.0, 0.0, 1.0]);
    }

    #[test]
    fn test_overhang_detected_with_zeroed_stl_normals() {
        // Regression: an overhang must still be found when the STL's stored normals are zeroed
        // (common from many exporters / binary STLs). The overhang loop previously trusted the
        // stored normal and silently skipped these facets, flipping a real warning to a false pass.
        let (fixtures_dir, printer, material) = get_fixtures_and_profiles();
        let src = std::fs::read_to_string(fixtures_dir.join("overhang_flange.stl")).unwrap();
        let zeroed: String = src
            .lines()
            .map(|line| {
                if line.trim_start().to_lowercase().starts_with("facet normal") {
                    "facet normal 0 0 0".to_string()
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        let temp_path = std::env::temp_dir().join("ppd_zeroed_overhang.stl");
        std::fs::write(&temp_path, zeroed).unwrap();

        let validator = StlModelValidator;
        let report = validator.validate_mesh(&temp_path, &printer, &material).unwrap();
        std::fs::remove_file(&temp_path).ok();

        assert!(
            report.issues.iter().any(|i| i.id == "OVERHANG_UNSUPPORTED"),
            "overhang must be detected even with zeroed STL normals; got: {:?}",
            report.issues.iter().map(|i| &i.id).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_model_resting_on_bed_within_tolerance_is_not_out_of_bounds() {
        // Regression: a model resting on the bed with sub-tolerance float/placement noise (Z min a
        // few hundredths of a mm below 0, well inside the below-bed tolerance) must NOT be flagged
        // out-of-bounds — while a model genuinely below the bed still must be.
        fn cube_stl(zmin: f32) -> String {
            let (x0, x1, y0, y1, z0, z1) = (10.0f32, 20.0, 10.0, 20.0, zmin, zmin + 10.0);
            let c = [
                [x0, y0, z0], [x1, y0, z0], [x1, y1, z0], [x0, y1, z0],
                [x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1],
            ];
            let faces: [([usize; 3], [usize; 3], [f32; 3]); 6] = [
                ([0, 3, 2], [0, 2, 1], [0.0, 0.0, -1.0]),
                ([4, 5, 6], [4, 6, 7], [0.0, 0.0, 1.0]),
                ([0, 1, 5], [0, 5, 4], [0.0, -1.0, 0.0]),
                ([2, 3, 7], [2, 7, 6], [0.0, 1.0, 0.0]),
                ([1, 2, 6], [1, 6, 5], [1.0, 0.0, 0.0]),
                ([0, 4, 7], [0, 7, 3], [-1.0, 0.0, 0.0]),
            ];
            let mut s = String::from("solid cube\n");
            for (a, b, n) in faces.iter() {
                for tri in [a, b] {
                    s.push_str(&format!(" facet normal {} {} {}\n  outer loop\n", n[0], n[1], n[2]));
                    for &idx in tri.iter() {
                        s.push_str(&format!(
                            "   vertex {:.4} {:.4} {:.4}\n",
                            c[idx][0], c[idx][1], c[idx][2]
                        ));
                    }
                    s.push_str("  endloop\n endfacet\n");
                }
            }
            s.push_str("endsolid cube\n");
            s
        }

        let (_fixtures, printer, material) = get_fixtures_and_profiles();
        let validator = StlModelValidator;

        let near = std::env::temp_dir().join("ppd_bed_within_tol.stl");
        std::fs::write(&near, cube_stl(-0.03)).unwrap();
        let r = validator.validate_mesh(&near, &printer, &material).unwrap();
        std::fs::remove_file(&near).ok();
        assert!(
            !r.issues.iter().any(|i| i.id == "MODEL_OUT_OF_BOUNDS"),
            "a model resting on the bed within tolerance must not be out-of-bounds; got: {:?}",
            r.issues.iter().map(|i| &i.id).collect::<Vec<_>>()
        );

        let below = std::env::temp_dir().join("ppd_bed_below.stl");
        std::fs::write(&below, cube_stl(-1.0)).unwrap();
        let r2 = validator.validate_mesh(&below, &printer, &material).unwrap();
        std::fs::remove_file(&below).ok();
        assert!(
            r2.issues.iter().any(|i| i.id == "MODEL_OUT_OF_BOUNDS"),
            "a model 1mm below the bed must still be out-of-bounds"
        );
    }

    #[test]
    fn test_validate_mesh_degenerate() {
        let (fixtures_dir, printer, material) = get_fixtures_and_profiles();
        let path = fixtures_dir.join("degenerate_triangle.stl");

        let validator = StlModelValidator;
        let report = validator.validate_mesh(&path, &printer, &material).unwrap();

        assert_eq!(report.status, ValidationStatus::Pass);
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.id == "DEGENERATE_TRIANGLES"));
    }

    #[test]
    fn test_validate_gcode_unsupported_file_type() {
        let (fixtures_dir, printer, material) = get_fixtures_and_profiles();
        let path = fixtures_dir.join("unsupported_file.txt");

        let validator = StandardGcodeValidator;
        let report = validator
            .validate_gcode(&path, &printer, &material)
            .unwrap();

        assert_eq!(report.status, ValidationStatus::Fail);
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.id == "UNSUPPORTED_FILE_TYPE"));
    }

    #[test]
    fn test_validate_gcode_unsafe_command() {
        let (fixtures_dir, printer, material) = get_fixtures_and_profiles();
        let path = fixtures_dir.join("unsafe_command.gcode");

        let validator = StandardGcodeValidator;
        let report = validator
            .validate_gcode(&path, &printer, &material)
            .unwrap();

        assert_eq!(report.status, ValidationStatus::Fail);
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.id == "UNSAFE_COMMAND_BLOCKED"));
    }

    #[test]
    fn test_validate_gcode_missing_homing() {
        let (fixtures_dir, printer, material) = get_fixtures_and_profiles();
        let path = fixtures_dir.join("missing_homing.gcode");

        let validator = StandardGcodeValidator;
        let report = validator
            .validate_gcode(&path, &printer, &material)
            .unwrap();

        assert_eq!(report.status, ValidationStatus::Warning);
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.id == "MISSING_HOMING"));
    }

    #[test]
    fn test_validate_gcode_cold_extrusion() {
        let (fixtures_dir, printer, material) = get_fixtures_and_profiles();
        let path = fixtures_dir.join("cold_extrusion.gcode");

        let validator = StandardGcodeValidator;
        let report = validator
            .validate_gcode(&path, &printer, &material)
            .unwrap();

        assert_eq!(report.status, ValidationStatus::Warning);
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.id == "COLD_EXTRUSION"));
    }

    #[test]
    fn test_validate_gcode_g92_and_m82_m83() {
        let (_, printer, material) = get_fixtures_and_profiles();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("g92_test.gcode");
        let gcode_content = "G28\n\
                             M109 S210\n\
                             G92 X200 Y200 Z100 E0\n\
                             G1 X210 Y210 Z110 E5\n\
                             M82\n\
                             G92 E0\n\
                             G1 E2\n";

        std::fs::write(&path, gcode_content).unwrap();
        let validator = StandardGcodeValidator;
        let report = validator
            .validate_gcode(&path, &printer, &material)
            .unwrap();

        assert_eq!(report.status, ValidationStatus::Pass);
        assert!(report.issues.is_empty());
    }
}
