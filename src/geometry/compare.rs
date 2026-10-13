use crate::csg::Mesh;
use std::collections::HashMap;

/// Global mass and topological statistics of a 3D mesh.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MeshStats {
    pub vertex_count: usize,
    pub face_count: usize,
    pub bounds_min: [f64; 3],
    pub bounds_max: [f64; 3],
    pub size: [f64; 3],
    pub volume: f64,
    pub surface_area: f64,
    pub centroid: [f64; 3],
    pub is_watertight: bool,
}

impl MeshStats {
    pub fn compute(mesh: &Mesh) -> Self {
        let vertex_count = mesh.vertices.len();
        let face_count = mesh.faces.len();

        if vertex_count == 0 || face_count == 0 {
            return Self {
                vertex_count,
                face_count,
                bounds_min: [0.0; 3],
                bounds_max: [0.0; 3],
                size: [0.0; 3],
                volume: 0.0,
                surface_area: 0.0,
                centroid: [0.0; 3],
                is_watertight: false,
            };
        }

        let mut b_min = [f64::INFINITY; 3];
        let mut b_max = [f64::NEG_INFINITY; 3];
        for v in &mesh.vertices {
            for i in 0..3 {
                if v[i] < b_min[i] {
                    b_min[i] = v[i];
                }
                if v[i] > b_max[i] {
                    b_max[i] = v[i];
                }
            }
        }
        let size = [
            (b_max[0] - b_min[0]).max(0.0),
            (b_max[1] - b_min[1]).max(0.0),
            (b_max[2] - b_min[2]).max(0.0),
        ];

        let mut signed_vol_sum = 0.0;
        let mut area_sum = 0.0;
        let mut centroid_vol_weighted = [0.0; 3];
        let mut centroid_area_weighted = [0.0; 3];

        let mut edge_counts: HashMap<(usize, usize), i32> = HashMap::new();

        for face in &mesh.faces {
            let p0 = mesh.vertices[face[0]];
            let p1 = mesh.vertices[face[1]];
            let p2 = mesh.vertices[face[2]];

            // Signed volume of tetrahedron formed by (0,0,0) and the triangle
            // V = 1/6 * det([p0, p1, p2]) = 1/6 * (p0 . (p1 x p2))
            let cross = [
                p1[1] * p2[2] - p1[2] * p2[1],
                p1[2] * p2[0] - p1[0] * p2[2],
                p1[0] * p2[1] - p1[1] * p2[0],
            ];
            let tet_vol = (p0[0] * cross[0] + p0[1] * cross[1] + p0[2] * cross[2]) / 6.0;
            signed_vol_sum += tet_vol;

            for i in 0..3 {
                centroid_vol_weighted[i] += (p0[i] + p1[i] + p2[i]) / 4.0 * tet_vol;
            }

            // Triangle surface area: 0.5 * |(p1 - p0) x (p2 - p0)|
            let u = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
            let v = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];
            let tri_cross = [
                u[1] * v[2] - u[2] * v[1],
                u[2] * v[0] - u[0] * v[2],
                u[0] * v[1] - u[1] * v[0],
            ];
            let tri_area = 0.5
                * (tri_cross[0] * tri_cross[0]
                    + tri_cross[1] * tri_cross[1]
                    + tri_cross[2] * tri_cross[2])
                    .sqrt();
            area_sum += tri_area;

            for i in 0..3 {
                centroid_area_weighted[i] += (p0[i] + p1[i] + p2[i]) / 3.0 * tri_area;
            }

            // Topology check (watertight edges)
            for k in 0..3 {
                let u = face[k];
                let w = face[(k + 1) % 3];
                let key = if u < w { (u, w) } else { (w, u) };
                *edge_counts.entry(key).or_insert(0) += 1;
            }
        }

        let abs_volume = signed_vol_sum.abs();
        let centroid = if abs_volume > 1e-6 {
            [
                centroid_vol_weighted[0] / signed_vol_sum,
                centroid_vol_weighted[1] / signed_vol_sum,
                centroid_vol_weighted[2] / signed_vol_sum,
            ]
        } else if area_sum > 1e-6 {
            [
                centroid_area_weighted[0] / area_sum,
                centroid_area_weighted[1] / area_sum,
                centroid_area_weighted[2] / area_sum,
            ]
        } else {
            [
                (b_min[0] + b_max[0]) / 2.0,
                (b_min[1] + b_max[1]) / 2.0,
                (b_min[2] + b_max[2]) / 2.0,
            ]
        };

        // Watertight: every edge is shared by exactly 2 faces
        let is_watertight =
            !edge_counts.is_empty() && edge_counts.values().all(|&count| count == 2);

        Self {
            vertex_count,
            face_count,
            bounds_min: b_min,
            bounds_max: b_max,
            size,
            volume: abs_volume,
            surface_area: area_sum,
            centroid,
            is_watertight,
        }
    }
}

/// Statistics of point-to-surface distance samples.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DirectionalHausdorff {
    pub sample_count: usize,
    pub max_dist: f64,
    pub mean_dist: f64,
    pub rms_dist: f64,
    pub p50: f64,
    pub p90: f64,
    pub p95: f64,
    pub p99: f64,
}

impl DirectionalHausdorff {
    pub fn compute(distances: &mut [f64]) -> Self {
        if distances.is_empty() {
            return Self {
                sample_count: 0,
                max_dist: 0.0,
                mean_dist: 0.0,
                rms_dist: 0.0,
                p50: 0.0,
                p90: 0.0,
                p95: 0.0,
                p99: 0.0,
            };
        }
        distances.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let n = distances.len();
        let max_dist = distances[n - 1];
        let sum: f64 = distances.iter().sum();
        let mean_dist = sum / (n as f64);
        let sum_sq: f64 = distances.iter().map(|d| d * d).sum();
        let rms_dist = (sum_sq / (n as f64)).sqrt();

        let p50 = distances[(n as f64 * 0.50) as usize];
        let p90 = distances[((n as f64 * 0.90) as usize).min(n - 1)];
        let p95 = distances[((n as f64 * 0.95) as usize).min(n - 1)];
        let p99 = distances[((n as f64 * 0.99) as usize).min(n - 1)];

        Self {
            sample_count: n,
            max_dist,
            mean_dist,
            rms_dist,
            p50,
            p90,
            p95,
            p99,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct HausdorffReport {
    pub a_to_b: DirectionalHausdorff,
    pub b_to_a: DirectionalHausdorff,
    pub symmetric_max: f64,
    pub symmetric_mean: f64,
    pub symmetric_rms: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ViewComparison {
    pub name: String,
    pub width: usize,
    pub height: usize,
    pub pixels_a: usize,
    pub pixels_b: usize,
    pub intersection_pixels: usize,
    pub union_pixels: usize,
    pub silhouette_iou: f64,
    pub depth_mae: f64,
    pub depth_max_err: f64,
    #[serde(skip)]
    pub rgb_image: Vec<u8>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub enum VerdictStatus {
    Pass,
    Warning,
    Fail,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ComparisonVerdict {
    pub status: VerdictStatus,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ComparisonReport {
    pub model_a_stats: MeshStats,
    pub model_b_stats: MeshStats,
    pub delta_bounds_min: [f64; 3],
    pub delta_bounds_max: [f64; 3],
    pub delta_size: [f64; 3],
    pub max_size_diff: f64,
    pub delta_volume: f64,
    pub volume_rel_diff_pct: f64,
    pub delta_surface_area: f64,
    pub area_rel_diff_pct: f64,
    pub centroid_distance: f64,
    pub hausdorff: HausdorffReport,
    pub views: Vec<ViewComparison>,
    pub verdict: ComparisonVerdict,
}

impl ComparisonReport {
    pub fn format_text_report(&self, name_a: &str, name_b: &str) -> String {
        let mut out = String::new();
        out.push_str(
            "================================================================================\n",
        );
        out.push_str(
            "             ACCURATE 3D MESH GEOMETRIC COMPARISON REPORT                     \n",
        );
        out.push_str(
            "================================================================================\n\n",
        );

        out.push_str(&format!("Model A (Reference): {}\n", name_a));
        out.push_str(&format!("Model B (Tested):    {}\n\n", name_b));

        out.push_str("--- 1. TOPOLOGY & MESH STATS ---\n");
        out.push_str(&format!(
            "{:<24} {:<24} {:<24}\n",
            "Property", "Model A", "Model B"
        ));
        out.push_str(&format!("{:-<72}\n", ""));
        out.push_str(&format!(
            "{:<24} {:<24} {:<24}\n",
            "Triangles", self.model_a_stats.face_count, self.model_b_stats.face_count
        ));
        out.push_str(&format!(
            "{:<24} {:<24} {:<24}\n",
            "Vertices", self.model_a_stats.vertex_count, self.model_b_stats.vertex_count
        ));
        out.push_str(&format!(
            "{:<24} {:<24} {:<24}\n\n",
            "Watertight (Manifold)",
            if self.model_a_stats.is_watertight {
                "YES"
            } else {
                "NO"
            },
            if self.model_b_stats.is_watertight {
                "YES"
            } else {
                "NO"
            }
        ));

        out.push_str("--- 2. BOUNDING BOX & SIZES (mm) ---\n");
        let sa = &self.model_a_stats.size;
        let sb = &self.model_b_stats.size;
        let ds = &self.delta_size;
        out.push_str(&format!(
            "Model A Size (X, Y, Z):  [{:.4}, {:.4}, {:.4}]\n",
            sa[0], sa[1], sa[2]
        ));
        out.push_str(&format!(
            "Model B Size (X, Y, Z):  [{:.4}, {:.4}, {:.4}]\n",
            sb[0], sb[1], sb[2]
        ));
        out.push_str(&format!(
            "Size Delta   (X, Y, Z):  [{:.4}, {:.4}, {:.4}]  (Max Diff: {:.4} mm)\n",
            ds[0], ds[1], ds[2], self.max_size_diff
        ));
        out.push_str(&format!(
            "Model A Bounds X: [{:.3} .. {:.3}], Y: [{:.3} .. {:.3}], Z: [{:.3} .. {:.3}]\n",
            self.model_a_stats.bounds_min[0],
            self.model_a_stats.bounds_max[0],
            self.model_a_stats.bounds_min[1],
            self.model_a_stats.bounds_max[1],
            self.model_a_stats.bounds_min[2],
            self.model_a_stats.bounds_max[2],
        ));
        out.push_str(&format!(
            "Model B Bounds X: [{:.3} .. {:.3}], Y: [{:.3} .. {:.3}], Z: [{:.3} .. {:.3}]\n\n",
            self.model_b_stats.bounds_min[0],
            self.model_b_stats.bounds_max[0],
            self.model_b_stats.bounds_min[1],
            self.model_b_stats.bounds_max[1],
            self.model_b_stats.bounds_min[2],
            self.model_b_stats.bounds_max[2],
        ));

        out.push_str("--- 3. MASS PROPERTIES (Volume, Surface Area, Centroid) ---\n");
        out.push_str(&format!(
            "Volume A:            {:.4} mm³\n",
            self.model_a_stats.volume
        ));
        out.push_str(&format!(
            "Volume B:            {:.4} mm³\n",
            self.model_b_stats.volume
        ));
        out.push_str(&format!(
            "Volume Delta:        {:.4} mm³ ({:.3}% relative)\n",
            self.delta_volume, self.volume_rel_diff_pct
        ));
        out.push_str(&format!(
            "Surface Area A:      {:.4} mm²\n",
            self.model_a_stats.surface_area
        ));
        out.push_str(&format!(
            "Surface Area B:      {:.4} mm²\n",
            self.model_b_stats.surface_area
        ));
        out.push_str(&format!(
            "Area Delta:          {:.4} mm² ({:.3}% relative)\n",
            self.delta_surface_area, self.area_rel_diff_pct
        ));
        out.push_str(&format!(
            "Centroid Distance:   {:.4} mm\n\n",
            self.centroid_distance
        ));

        out.push_str("--- 4. TWO-SIDED SAMPLED HAUSDORFF DISTANCE (mm) ---\n");
        out.push_str(&format!(
            "Samples per mesh:    {}\n",
            self.hausdorff.a_to_b.sample_count
        ));
        out.push_str(&format!(
            "Symmetric Max (Hausdorff): {:.5} mm\n",
            self.hausdorff.symmetric_max
        ));
        out.push_str(&format!(
            "Symmetric Mean Distance:   {:.5} mm\n",
            self.hausdorff.symmetric_mean
        ));
        out.push_str(&format!(
            "Symmetric RMS Distance:    {:.5} mm\n",
            self.hausdorff.symmetric_rms
        ));
        out.push_str(&format!(
            "A -> B Percentiles:  p50: {:.5} mm | p90: {:.5} mm | p95: {:.5} mm | p99: {:.5} mm\n",
            self.hausdorff.a_to_b.p50,
            self.hausdorff.a_to_b.p90,
            self.hausdorff.a_to_b.p95,
            self.hausdorff.a_to_b.p99
        ));
        out.push_str(&format!(
            "B -> A Percentiles:  p50: {:.5} mm | p90: {:.5} mm | p95: {:.5} mm | p99: {:.5} mm\n\n",
            self.hausdorff.b_to_a.p50,
            self.hausdorff.b_to_a.p90,
            self.hausdorff.b_to_a.p95,
            self.hausdorff.b_to_a.p99
        ));

        out.push_str("--- 5. MULTI-VIEW ORTHOGRAPHIC PROJECTIONS & DEPTH (IoU) ---\n");
        for v in &self.views {
            out.push_str(&format!(
                "View: {:<16} | Silhouette IoU: {:>6.2}% | Depth MAE: {:.5} mm | Depth MaxErr: {:.4} mm\n",
                v.name,
                v.silhouette_iou * 100.0,
                v.depth_mae,
                v.depth_max_err
            ));
        }
        out.push_str("\n");

        out.push_str("--- 6. FINAL VERDICT ---\n");
        match self.verdict.status {
            VerdictStatus::Pass => {
                out.push_str(">>> RESULT: [PASS] Models are geometrically equivalent within tolerances! <<<\n");
            }
            VerdictStatus::Warning => {
                out.push_str(">>> RESULT: [WARNING] Minor geometric deviations detected: <<<\n");
                for r in &self.verdict.reasons {
                    out.push_str(&format!("  - {r}\n"));
                }
            }
            VerdictStatus::Fail => {
                out.push_str(">>> RESULT: [FAIL] Significant geometric discrepancy! <<<\n");
                for r in &self.verdict.reasons {
                    out.push_str(&format!("  - {r}\n"));
                }
            }
        }
        out.push_str(
            "================================================================================\n",
        );

        out
    }
}

// -----------------------------------------------------------------------------
// Geometric algorithms: Point-Triangle Distance, Spatial Grid, Surface Sampling
// -----------------------------------------------------------------------------

/// Exact 3D closest point on triangle (p0, p1, p2) to query point p.
/// Returns distance from p to triangle.
pub fn point_triangle_distance(p: [f64; 3], a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> f64 {
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let ap = [p[0] - a[0], p[1] - a[1], p[2] - a[2]];

    let d1 = ab[0] * ap[0] + ab[1] * ap[1] + ab[2] * ap[2];
    let d2 = ac[0] * ap[0] + ac[1] * ap[1] + ac[2] * ap[2];
    if d1 <= 0.0 && d2 <= 0.0 {
        return dist3(p, a);
    }

    let bp = [p[0] - b[0], p[1] - b[1], p[2] - b[2]];
    let d3 = ab[0] * bp[0] + ab[1] * bp[1] + ab[2] * bp[2];
    let d4 = ac[0] * bp[0] + ac[1] * bp[1] + ac[2] * bp[2];
    if d3 >= 0.0 && d4 <= d3 {
        return dist3(p, b);
    }

    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        let proj = [a[0] + v * ab[0], a[1] + v * ab[1], a[2] + v * ab[2]];
        return dist3(p, proj);
    }

    let cp = [p[0] - c[0], p[1] - c[1], p[2] - c[2]];
    let d5 = ab[0] * cp[0] + ab[1] * cp[1] + ab[2] * cp[2];
    let d6 = ac[0] * cp[0] + ac[1] * cp[1] + ac[2] * cp[2];
    if d6 >= 0.0 && d5 <= d6 {
        return dist3(p, c);
    }

    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        let proj = [a[0] + w * ac[0], a[1] + w * ac[1], a[2] + w * ac[2]];
        return dist3(p, proj);
    }

    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let bc = [c[0] - b[0], c[1] - b[1], c[2] - b[2]];
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        let proj = [b[0] + w * bc[0], b[1] + w * bc[1], b[2] + w * bc[2]];
        return dist3(p, proj);
    }

    let denom = 1.0 / (va + vb + vc);
    let v = vb * denom;
    let w = vc * denom;
    let proj = [
        a[0] + ab[0] * v + ac[0] * w,
        a[1] + ab[1] * v + ac[1] * w,
        a[2] + ab[2] * v + ac[2] * w,
    ];
    dist3(p, proj)
}

#[inline(always)]
fn dist3(a: [f64; 3], b: [f64; 3]) -> f64 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// Uniform Spatial Grid for accelerated distance queries against a Mesh.
pub struct SpatialGrid<'a> {
    mesh: &'a Mesh,
    b_min: [f64; 3],
    cell_size: [f64; 3],
    dims: [usize; 3],
    grid: Vec<Vec<usize>>,
}

impl<'a> SpatialGrid<'a> {
    pub fn build(mesh: &'a Mesh) -> Self {
        let mut b_min = [f64::INFINITY; 3];
        let mut b_max = [f64::NEG_INFINITY; 3];
        for v in &mesh.vertices {
            for i in 0..3 {
                b_min[i] = b_min[i].min(v[i]);
                b_max[i] = b_max[i].max(v[i]);
            }
        }
        for i in 0..3 {
            b_min[i] -= 0.1;
            b_max[i] += 0.1;
        }

        let face_count = mesh.faces.len();
        let target_cells = (face_count as f64).cbrt().max(4.0).min(48.0) as usize;
        let dims = [target_cells, target_cells, target_cells];

        let cell_size = [
            (b_max[0] - b_min[0]) / (dims[0] as f64),
            (b_max[1] - b_min[1]) / (dims[1] as f64),
            (b_max[2] - b_min[2]) / (dims[2] as f64),
        ];

        let total_cells = dims[0] * dims[1] * dims[2];
        let mut grid: Vec<Vec<usize>> = vec![Vec::new(); total_cells];

        for (face_idx, face) in mesh.faces.iter().enumerate() {
            let p0 = mesh.vertices[face[0]];
            let p1 = mesh.vertices[face[1]];
            let p2 = mesh.vertices[face[2]];

            let f_min = [
                p0[0].min(p1[0]).min(p2[0]),
                p0[1].min(p1[1]).min(p2[1]),
                p0[2].min(p1[2]).min(p2[2]),
            ];
            let f_max = [
                p0[0].max(p1[0]).max(p2[0]),
                p0[1].max(p1[1]).max(p2[1]),
                p0[2].max(p1[2]).max(p2[2]),
            ];

            let c_min = [
                ((f_min[0] - b_min[0]) / cell_size[0]).floor().max(0.0) as usize,
                ((f_min[1] - b_min[1]) / cell_size[1]).floor().max(0.0) as usize,
                ((f_min[2] - b_min[2]) / cell_size[2]).floor().max(0.0) as usize,
            ];
            let c_max = [
                ((f_max[0] - b_min[0]) / cell_size[0])
                    .floor()
                    .min(dims[0] as f64 - 1.0) as usize,
                ((f_max[1] - b_min[1]) / cell_size[1])
                    .floor()
                    .min(dims[1] as f64 - 1.0) as usize,
                ((f_max[2] - b_min[2]) / cell_size[2])
                    .floor()
                    .min(dims[2] as f64 - 1.0) as usize,
            ];

            for x in c_min[0]..=c_max[0] {
                for y in c_min[1]..=c_max[1] {
                    for z in c_min[2]..=c_max[2] {
                        let idx = (x * dims[1] + y) * dims[2] + z;
                        grid[idx].push(face_idx);
                    }
                }
            }
        }

        Self {
            mesh,
            b_min,
            cell_size,
            dims,
            grid,
        }
    }

    pub fn closest_distance(&self, p: [f64; 3]) -> f64 {
        if self.mesh.faces.is_empty() {
            return f64::INFINITY;
        }

        let cx = ((p[0] - self.b_min[0]) / self.cell_size[0])
            .floor()
            .clamp(0.0, self.dims[0] as f64 - 1.0) as isize;
        let cy = ((p[1] - self.b_min[1]) / self.cell_size[1])
            .floor()
            .clamp(0.0, self.dims[1] as f64 - 1.0) as isize;
        let cz = ((p[2] - self.b_min[2]) / self.cell_size[2])
            .floor()
            .clamp(0.0, self.dims[2] as f64 - 1.0) as isize;

        let mut best_dist = f64::INFINITY;

        // Check the containing cell first
        let center_idx =
            ((cx as usize) * self.dims[1] + (cy as usize)) * self.dims[2] + (cz as usize);
        for &face_idx in &self.grid[center_idx] {
            let f = self.mesh.faces[face_idx];
            let d = point_triangle_distance(
                p,
                self.mesh.vertices[f[0]],
                self.mesh.vertices[f[1]],
                self.mesh.vertices[f[2]],
            );
            if d < best_dist {
                best_dist = d;
            }
        }

        // Search expanding shells of cells until cell distance > best_dist
        let max_r = (self.dims[0].max(self.dims[1]).max(self.dims[2])) as isize;
        for r in 1..=max_r {
            let r_f = (r - 1) as f64;
            let min_cell_dist = r_f
                * (self.cell_size[0]
                    .min(self.cell_size[1])
                    .min(self.cell_size[2]));
            if min_cell_dist > best_dist {
                break;
            }

            for dx in -r..=r {
                for dy in -r..=r {
                    for dz in -r..=r {
                        if dx.abs() != r && dy.abs() != r && dz.abs() != r {
                            continue;
                        }
                        let x = cx + dx;
                        let y = cy + dy;
                        let z = cz + dz;
                        if x < 0
                            || x >= self.dims[0] as isize
                            || y < 0
                            || y >= self.dims[1] as isize
                            || z < 0
                            || z >= self.dims[2] as isize
                        {
                            continue;
                        }
                        let idx = ((x as usize) * self.dims[1] + (y as usize)) * self.dims[2]
                            + (z as usize);
                        for &face_idx in &self.grid[idx] {
                            let f = self.mesh.faces[face_idx];
                            let d = point_triangle_distance(
                                p,
                                self.mesh.vertices[f[0]],
                                self.mesh.vertices[f[1]],
                                self.mesh.vertices[f[2]],
                            );
                            if d < best_dist {
                                best_dist = d;
                            }
                        }
                    }
                }
            }
        }

        if best_dist.is_infinite() {
            // Fallback: brute force
            for face in &self.mesh.faces {
                let d = point_triangle_distance(
                    p,
                    self.mesh.vertices[face[0]],
                    self.mesh.vertices[face[1]],
                    self.mesh.vertices[face[2]],
                );
                if d < best_dist {
                    best_dist = d;
                }
            }
        }

        best_dist
    }
}

/// Generate uniformly distributed random points on the mesh surface.
pub fn sample_mesh_surface(mesh: &Mesh, sample_count: usize, seed: u64) -> Vec<[f64; 3]> {
    if mesh.faces.is_empty() || sample_count == 0 {
        return Vec::new();
    }

    let mut cumulative_areas = Vec::with_capacity(mesh.faces.len());
    let mut total_area = 0.0;
    for face in &mesh.faces {
        let p0 = mesh.vertices[face[0]];
        let p1 = mesh.vertices[face[1]];
        let p2 = mesh.vertices[face[2]];
        let u = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
        let v = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];
        let cross = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        let area = 0.5 * (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt();
        total_area += area;
        cumulative_areas.push(total_area);
    }

    if total_area <= 1e-12 {
        return Vec::new();
    }

    // Fast deterministic PRNG (xorshift64)
    let mut rng = seed;
    let mut next_float = || -> f64 {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        (rng as f64) / (u64::MAX as f64)
    };

    let mut samples = Vec::with_capacity(sample_count);
    for _ in 0..sample_count {
        let r_area = next_float() * total_area;
        let face_idx = match cumulative_areas.binary_search_by(|val| {
            val.partial_cmp(&r_area)
                .unwrap_or(std::cmp::Ordering::Equal)
        }) {
            Ok(i) => i,
            Err(i) => i.min(mesh.faces.len() - 1),
        };

        let face = mesh.faces[face_idx];
        let p0 = mesh.vertices[face[0]];
        let p1 = mesh.vertices[face[1]];
        let p2 = mesh.vertices[face[2]];

        let mut u1 = next_float();
        let mut u2 = next_float();
        if u1 + u2 > 1.0 {
            u1 = 1.0 - u1;
            u2 = 1.0 - u2;
        }

        let sample = [
            p0[0] + u1 * (p1[0] - p0[0]) + u2 * (p2[0] - p0[0]),
            p0[1] + u1 * (p1[1] - p0[1]) + u2 * (p2[1] - p0[1]),
            p0[2] + u1 * (p1[2] - p0[2]) + u2 * (p2[2] - p0[2]),
        ];
        samples.push(sample);
    }

    samples
}

// -----------------------------------------------------------------------------
// Multi-view Orthographic Rasterizer and Depth Buffer Analyzer
// -----------------------------------------------------------------------------

#[derive(Clone, Copy)]
pub enum OrthoView {
    Top,   // Project XY plane, camera along -Z (Z is depth)
    Front, // Project XZ plane, camera along +Y (Y is depth)
    Side,  // Project YZ plane, camera along +X (X is depth)
}

#[allow(dead_code)]
struct DepthBuffer {
    width: usize,
    height: usize,
    depth: Vec<f64>,
    mask: Vec<bool>,
    shading: Vec<f64>,
}

impl DepthBuffer {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            depth: vec![f64::NEG_INFINITY; width * height],
            mask: vec![false; width * height],
            shading: vec![0.0; width * height],
        }
    }
}

fn rasterize_view(
    mesh: &Mesh,
    view: OrthoView,
    res: usize,
    b_min: [f64; 3],
    b_max: [f64; 3],
) -> DepthBuffer {
    let mut buf = DepthBuffer::new(res, res);
    let (ax_u, ax_v, ax_d, flip_d) = match view {
        OrthoView::Top => (0, 1, 2, false),
        OrthoView::Front => (0, 2, 1, false),
        OrthoView::Side => (1, 2, 0, false),
    };

    let u_min = b_min[ax_u];
    let u_max = b_max[ax_u];
    let v_min = b_min[ax_v];
    let v_max = b_max[ax_v];

    let span_u = (u_max - u_min).max(1e-4);
    let span_v = (v_max - v_min).max(1e-4);

    for face in &mesh.faces {
        let p0 = mesh.vertices[face[0]];
        let p1 = mesh.vertices[face[1]];
        let p2 = mesh.vertices[face[2]];

        let to_screen = |p: [f64; 3]| -> ([f64; 2], f64) {
            let u = (p[ax_u] - u_min) / span_u * ((res - 1) as f64);
            let v = (p[ax_v] - v_min) / span_v * ((res - 1) as f64);
            let d = if flip_d { -p[ax_d] } else { p[ax_d] };
            ([u, v], d)
        };

        let (s0, d0) = to_screen(p0);
        let (s1, d1) = to_screen(p1);
        let (s2, d2) = to_screen(p2);

        // Compute simple directional normal shading
        let e1 = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
        let e2 = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];
        let n = [
            e1[1] * e2[2] - e1[2] * e2[1],
            e1[2] * e2[0] - e1[0] * e2[2],
            e1[0] * e2[1] - e1[1] * e2[0],
        ];
        let n_len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-8);
        let light = match view {
            OrthoView::Top => [0.3, 0.4, 0.86],
            OrthoView::Front => [0.4, 0.86, 0.3],
            OrthoView::Side => [0.86, 0.3, 0.4],
        };
        let dot = ((n[0] * light[0] + n[1] * light[1] + n[2] * light[2]) / n_len).abs();
        let shade = (0.3 + 0.7 * dot).clamp(0.0, 1.0);

        // 2D bounding box
        let min_x = s0[0].min(s1[0]).min(s2[0]).floor().max(0.0) as usize;
        let max_x = s0[0].max(s1[0]).max(s2[0]).ceil().min(res as f64 - 1.0) as usize;
        let min_y = s0[1].min(s1[1]).min(s2[1]).floor().max(0.0) as usize;
        let max_y = s0[1].max(s1[1]).max(s2[1]).ceil().min(res as f64 - 1.0) as usize;

        // Area in 2D
        let area2 = (s1[0] - s0[0]) * (s2[1] - s0[1]) - (s1[1] - s0[1]) * (s2[0] - s0[0]);
        if area2.abs() < 1e-6 {
            continue;
        }

        for y in min_y..=max_y {
            let py = y as f64;
            for x in min_x..=max_x {
                let px = x as f64;
                let w0 = ((s1[0] - px) * (s2[1] - py) - (s1[1] - py) * (s2[0] - px)) / area2;
                let w1 = ((s2[0] - px) * (s0[1] - py) - (s2[1] - py) * (s0[0] - px)) / area2;
                let w2 = 1.0 - w0 - w1;

                if w0 >= -1e-4 && w1 >= -1e-4 && w2 >= -1e-4 {
                    let depth = w0 * d0 + w1 * d1 + w2 * d2;
                    let idx = y * res + x;
                    if depth > buf.depth[idx] {
                        buf.depth[idx] = depth;
                        buf.mask[idx] = true;
                        buf.shading[idx] = shade;
                    }
                }
            }
        }
    }

    buf
}

pub fn compare_view(
    mesh_a: &Mesh,
    mesh_b: &Mesh,
    view: OrthoView,
    view_name: &str,
    res: usize,
    common_b_min: [f64; 3],
    common_b_max: [f64; 3],
) -> ViewComparison {
    let buf_a = rasterize_view(mesh_a, view, res, common_b_min, common_b_max);
    let buf_b = rasterize_view(mesh_b, view, res, common_b_min, common_b_max);

    let mut pixels_a = 0;
    let mut pixels_b = 0;
    let mut intersection_pixels = 0;
    let mut union_pixels = 0;
    let mut depth_diff_sum = 0.0;
    let mut depth_max_err = 0.0f64;

    // Output composite image: 3 panels wide (Model A, Model B, Visual Diff)
    // Dimension: (res * 3) x res
    let img_w = res * 3;
    let img_h = res;
    let mut rgb_image = vec![25u8; img_w * img_h * 3]; // dark gray background

    for y in 0..res {
        for x in 0..res {
            let idx = y * res + x;
            let in_a = buf_a.mask[idx];
            let in_b = buf_b.mask[idx];

            if in_a {
                pixels_a += 1;
            }
            if in_b {
                pixels_b += 1;
            }

            if in_a || in_b {
                union_pixels += 1;
            }

            // Panel 0: Model A (Golden / Amber rendering)
            let p0_idx = ((res - 1 - y) * img_w + x) * 3;
            if in_a {
                let s = buf_a.shading[idx];
                rgb_image[p0_idx] = (220.0 * s) as u8;
                rgb_image[p0_idx + 1] = (180.0 * s) as u8;
                rgb_image[p0_idx + 2] = (70.0 * s) as u8;
            }

            // Panel 1: Model B (Teal / Cyan rendering)
            let p1_idx = ((res - 1 - y) * img_w + (res + x)) * 3;
            if in_b {
                let s = buf_b.shading[idx];
                rgb_image[p1_idx] = (60.0 * s) as u8;
                rgb_image[p1_idx + 1] = (190.0 * s) as u8;
                rgb_image[p1_idx + 2] = (230.0 * s) as u8;
            }

            // Panel 2: Diff View
            // Green: common silhouette, with color gradient depending on depth error.
            // Red: present only in Model A (Missing in B)
            // Blue: present only in Model B (Excess in B)
            let p2_idx = ((res - 1 - y) * img_w + (res * 2 + x)) * 3;
            if in_a && in_b {
                intersection_pixels += 1;
                let d_err = (buf_a.depth[idx] - buf_b.depth[idx]).abs();
                depth_diff_sum += d_err;
                if d_err > depth_max_err {
                    depth_max_err = d_err;
                }

                if d_err < 0.05 {
                    // Match: Bright Emerald Green
                    rgb_image[p2_idx] = 40;
                    rgb_image[p2_idx + 1] = 220;
                    rgb_image[p2_idx + 2] = 80;
                } else if d_err < 0.20 {
                    // Minor depth error: Yellow / Lime
                    rgb_image[p2_idx] = 220;
                    rgb_image[p2_idx + 1] = 220;
                    rgb_image[p2_idx + 2] = 40;
                } else {
                    // Significant depth error: Orange
                    rgb_image[p2_idx] = 255;
                    rgb_image[p2_idx + 1] = 130;
                    rgb_image[p2_idx + 2] = 20;
                }
            } else if in_a {
                // Missing in B: Bright Red
                rgb_image[p2_idx] = 240;
                rgb_image[p2_idx + 1] = 40;
                rgb_image[p2_idx + 2] = 40;
            } else if in_b {
                // Extra in B: Bright Blue
                rgb_image[p2_idx] = 40;
                rgb_image[p2_idx + 1] = 110;
                rgb_image[p2_idx + 2] = 255;
            }
        }
    }

    let silhouette_iou = if union_pixels > 0 {
        intersection_pixels as f64 / union_pixels as f64
    } else {
        1.0
    };

    let depth_mae = if intersection_pixels > 0 {
        depth_diff_sum / (intersection_pixels as f64)
    } else {
        0.0
    };

    ViewComparison {
        name: view_name.to_string(),
        width: img_w,
        height: img_h,
        pixels_a,
        pixels_b,
        intersection_pixels,
        union_pixels,
        silhouette_iou,
        depth_mae,
        depth_max_err,
        rgb_image,
    }
}

/// Run exhaustive geometric comparison between Mesh A (Reference) and Mesh B (Tested).
pub fn compare_meshes(mesh_a: &Mesh, mesh_b: &Mesh, sample_count: usize) -> ComparisonReport {
    let stats_a = MeshStats::compute(mesh_a);
    let stats_b = MeshStats::compute(mesh_b);

    let delta_bounds_min = [
        (stats_a.bounds_min[0] - stats_b.bounds_min[0]).abs(),
        (stats_a.bounds_min[1] - stats_b.bounds_min[1]).abs(),
        (stats_a.bounds_min[2] - stats_b.bounds_min[2]).abs(),
    ];
    let delta_bounds_max = [
        (stats_a.bounds_max[0] - stats_b.bounds_max[0]).abs(),
        (stats_a.bounds_max[1] - stats_b.bounds_max[1]).abs(),
        (stats_a.bounds_max[2] - stats_b.bounds_max[2]).abs(),
    ];
    let delta_size = [
        (stats_a.size[0] - stats_b.size[0]).abs(),
        (stats_a.size[1] - stats_b.size[1]).abs(),
        (stats_a.size[2] - stats_b.size[2]).abs(),
    ];
    let max_size_diff = delta_size[0].max(delta_size[1]).max(delta_size[2]);

    let delta_volume = (stats_a.volume - stats_b.volume).abs();
    let volume_rel_diff_pct = if stats_a.volume > 1e-6 {
        (delta_volume / stats_a.volume) * 100.0
    } else {
        0.0
    };

    let delta_surface_area = (stats_a.surface_area - stats_b.surface_area).abs();
    let area_rel_diff_pct = if stats_a.surface_area > 1e-6 {
        (delta_surface_area / stats_a.surface_area) * 100.0
    } else {
        0.0
    };

    let centroid_distance = dist3(stats_a.centroid, stats_b.centroid);

    // Two-sided Hausdorff distance calculation
    let grid_a = SpatialGrid::build(mesh_a);
    let grid_b = SpatialGrid::build(mesh_b);

    let samples_a = sample_mesh_surface(mesh_a, sample_count, 123456789);
    let samples_b = sample_mesh_surface(mesh_b, sample_count, 987654321);

    let mut dists_a_to_b = samples_a
        .iter()
        .map(|&p| grid_b.closest_distance(p))
        .collect::<Vec<_>>();
    let mut dists_b_to_a = samples_b
        .iter()
        .map(|&p| grid_a.closest_distance(p))
        .collect::<Vec<_>>();

    let h_a_to_b = DirectionalHausdorff::compute(&mut dists_a_to_b);
    let h_b_to_a = DirectionalHausdorff::compute(&mut dists_b_to_a);

    let symmetric_max = h_a_to_b.max_dist.max(h_b_to_a.max_dist);
    let symmetric_mean = (h_a_to_b.mean_dist + h_b_to_a.mean_dist) / 2.0;
    let symmetric_rms =
        ((h_a_to_b.rms_dist * h_a_to_b.rms_dist + h_b_to_a.rms_dist * h_b_to_a.rms_dist) / 2.0)
            .sqrt();

    let hausdorff = HausdorffReport {
        a_to_b: h_a_to_b,
        b_to_a: h_b_to_a,
        symmetric_max,
        symmetric_mean,
        symmetric_rms,
    };

    // Multi-view projections
    let mut common_b_min = [
        stats_a.bounds_min[0].min(stats_b.bounds_min[0]),
        stats_a.bounds_min[1].min(stats_b.bounds_min[1]),
        stats_a.bounds_min[2].min(stats_b.bounds_min[2]),
    ];
    let mut common_b_max = [
        stats_a.bounds_max[0].max(stats_b.bounds_max[0]),
        stats_a.bounds_max[1].max(stats_b.bounds_max[1]),
        stats_a.bounds_max[2].max(stats_b.bounds_max[2]),
    ];
    // Add 5% padding
    for i in 0..3 {
        let span = (common_b_max[i] - common_b_min[i]).max(1.0);
        common_b_min[i] -= span * 0.05;
        common_b_max[i] += span * 0.05;
    }

    let res = 384;
    let views = vec![
        compare_view(
            mesh_a,
            mesh_b,
            OrthoView::Top,
            "Top (Z-)",
            res,
            common_b_min,
            common_b_max,
        ),
        compare_view(
            mesh_a,
            mesh_b,
            OrthoView::Front,
            "Front (Y+)",
            res,
            common_b_min,
            common_b_max,
        ),
        compare_view(
            mesh_a,
            mesh_b,
            OrthoView::Side,
            "Side (X+)",
            res,
            common_b_min,
            common_b_max,
        ),
    ];

    // Evaluate verdict
    let mut reasons = Vec::new();
    let mut is_fail = false;
    let mut is_warning = false;

    if max_size_diff > 0.20 {
        reasons.push(format!(
            "Significant bounding size difference: {:.3} mm (limit: 0.20 mm)",
            max_size_diff
        ));
        is_fail = true;
    } else if max_size_diff > 0.05 {
        reasons.push(format!(
            "Minor bounding size difference: {:.3} mm",
            max_size_diff
        ));
        is_warning = true;
    }

    if volume_rel_diff_pct > 2.0 {
        reasons.push(format!(
            "Volume discrepancy: {:.2}% (limit: 2.0%)",
            volume_rel_diff_pct
        ));
        is_fail = true;
    } else if volume_rel_diff_pct > 0.5 {
        reasons.push(format!(
            "Slight volume discrepancy: {:.2}%",
            volume_rel_diff_pct
        ));
        is_warning = true;
    }

    if symmetric_max > 0.35 {
        reasons.push(format!(
            "Max Hausdorff distance too large: {:.3} mm (limit: 0.35 mm)",
            symmetric_max
        ));
        is_fail = true;
    } else if symmetric_max > 0.15 {
        reasons.push(format!(
            "Moderate Hausdorff distance: {:.3} mm",
            symmetric_max
        ));
        is_warning = true;
    }

    if symmetric_mean > 0.05 {
        reasons.push(format!(
            "Mean surface distance high: {:.4} mm (limit: 0.05 mm)",
            symmetric_mean
        ));
        is_fail = true;
    }

    for v in &views {
        if v.silhouette_iou < 0.95 {
            reasons.push(format!(
                "{} Silhouette IoU low: {:.2}% (limit: 95.0%)",
                v.name,
                v.silhouette_iou * 100.0
            ));
            is_fail = true;
        } else if v.silhouette_iou < 0.985 {
            reasons.push(format!(
                "{} Silhouette IoU slightly degraded: {:.2}%",
                v.name,
                v.silhouette_iou * 100.0
            ));
            is_warning = true;
        }
    }

    let status = if is_fail {
        VerdictStatus::Fail
    } else if is_warning {
        VerdictStatus::Warning
    } else {
        VerdictStatus::Pass
    };

    ComparisonReport {
        model_a_stats: stats_a,
        model_b_stats: stats_b,
        delta_bounds_min,
        delta_bounds_max,
        delta_size,
        max_size_diff,
        delta_volume,
        volume_rel_diff_pct,
        delta_surface_area,
        area_rel_diff_pct,
        centroid_distance,
        hausdorff,
        views,
        verdict: ComparisonVerdict { status, reasons },
    }
}
