// Simple CSG module using mesh representation
// Uses nalgebra for math, outputs to OBJ/STL

use nalgebra::Matrix4;
use manifold3d::Manifold;
// use crate::interpreter::value::Value;
// use crate::ast::Span;
// use anyhow::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BooleanOp {
    Union,
    Difference,
    Intersection,
}

#[derive(Debug, Clone)]
pub enum CsgNode {
    Primitive(Primitive),
    Transform {
        child: Box<CsgNode>,
        translate: Option<[f64; 3]>,
        rotate: Option<[f64; 3]>,
        scale: Option<[f64; 3]>,
        mirror: Option<[f64; 3]>,
        matrix: Option<Matrix4<f64>>,
    },
    Boolean {
        op: BooleanOp,
        left: Box<CsgNode>,
        right: Box<CsgNode>,
    },
    Hull {
        children: Vec<CsgNode>,
    },
    Minkowski {
        children: Vec<CsgNode>,
    },
    Color {
        child: Box<CsgNode>,
        color: [f64; 4],
    },
    LinearExtrude {
        child: Box<CsgNode>,
        height: f64,
        center: bool,
    },
    RotateExtrude {
        child: Box<CsgNode>,
        angle: f64,
        segments: usize,
    },
}

#[derive(Debug, Clone)]
pub enum Primitive {
    Cube { size: [f64; 3], center: bool },
    Sphere { r: f64, d: Option<f64>, fn_: Option<usize>, fa: Option<f64>, fs: Option<f64> },
    Cylinder { h: f64, r1: f64, r2: f64, d1: Option<f64>, d2: Option<f64>, center: bool, fn_: Option<usize>, fa: Option<f64>, fs: Option<f64> },
    Polyhedron { points: Vec<[f64; 3]>, faces: Vec<Vec<usize>> },
    Square { size: [f64; 2], center: bool },
    Circle { r: f64, d: Option<f64>, fn_: Option<usize>, fa: Option<f64>, fs: Option<f64> },
    Polygon { points: Vec<[f64; 2]>, paths: Vec<Vec<usize>> },
    Text { text: String, size: f64, font: String, halign: String, valign: String },
    Import { file: String, convexity: usize },
}

#[derive(Debug, Clone)]
pub struct Mesh {
    pub vertices: Vec<[f64; 3]>,
    pub faces: Vec<[usize; 3]>,
    pub normals: Vec<[f64; 3]>,
}

impl Mesh {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            faces: Vec::new(),
            normals: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty() || self.faces.is_empty()
    }

    fn valid_face(&self, face: &[usize; 3]) -> bool {
        if face.iter().any(|&index| index >= self.vertices.len()) {
            return false;
        }
        let a = self.vertices[face[0]];
        let b = self.vertices[face[1]];
        let c = self.vertices[face[2]];
        if !a.iter().chain(b.iter()).chain(c.iter()).all(|value| value.is_finite()) {
            return false;
        }
        let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let cross = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        cross.iter().map(|value| value * value).sum::<f64>() > f64::EPSILON
    }

    pub fn from_stl(data: &[u8]) -> Result<Self, String> {
            let mut mesh = Mesh::new();
            if data.len() >= 84 {
                let count = u32::from_le_bytes(data[80..84].try_into().unwrap()) as usize;
                if 84 + count.saturating_mul(50) == data.len() {
                    for triangle in data[84..].chunks_exact(50) {
                        let mut indices = [0; 3];
                        for (index, vertex) in indices.iter_mut().enumerate() {
                            let offset = 12 + index * 12;
                            let point = [
                                f32::from_le_bytes(triangle[offset..offset + 4].try_into().unwrap()) as f64,
                                f32::from_le_bytes(triangle[offset + 4..offset + 8].try_into().unwrap()) as f64,
                                f32::from_le_bytes(triangle[offset + 8..offset + 12].try_into().unwrap()) as f64,
                            ];
                            *vertex = Self::stl_vertex(&mut mesh.vertices, point);
                        }
                        mesh.faces.push(indices);
                    }
                    mesh.normals = vec![[0.0, 0.0, 1.0]; mesh.vertices.len()];
                    return Ok(mesh);
                }
            }

            let text = std::str::from_utf8(data)
                .map_err(|_| "STL is neither valid binary nor UTF-8 ASCII".to_string())?;
            let mut triangle = Vec::new();
            for line in text.lines() {
                let fields: Vec<_> = line.split_whitespace().collect();
                if fields.len() == 4 && fields[0].eq_ignore_ascii_case("vertex") {
                    let point = [
                        fields[1].parse().map_err(|_| "invalid STL vertex")?,
                        fields[2].parse().map_err(|_| "invalid STL vertex")?,
                        fields[3].parse().map_err(|_| "invalid STL vertex")?,
                    ];
                    triangle.push(Self::stl_vertex(&mut mesh.vertices, point));
                    if triangle.len() == 3 {
                        mesh.faces.push([triangle[0], triangle[1], triangle[2]]);
                        triangle.clear();
                    }
                }
            }
            if mesh.faces.is_empty() {
                return Err("STL contains no triangles".into());
            }
            mesh.normals = vec![[0.0, 0.0, 1.0]; mesh.vertices.len()];
            Ok(mesh)
    }

    pub fn from_obj(data: &[u8]) -> Result<Self, String> {
        let text = std::str::from_utf8(data)
            .map_err(|_| "OBJ is not valid UTF-8".to_string())?;
        let mut mesh = Mesh::new();
        for line in text.lines() {
            let mut fields = line.split_whitespace();
            match fields.next() {
                Some("v") => {
                    let coordinates: Vec<_> = fields.collect();
                    if coordinates.len() < 3 {
                        return Err("OBJ vertex must have three coordinates".into());
                    }
                    mesh.vertices.push([
                        coordinates[0].parse().map_err(|_| "invalid OBJ vertex")?,
                        coordinates[1].parse().map_err(|_| "invalid OBJ vertex")?,
                        coordinates[2].parse().map_err(|_| "invalid OBJ vertex")?,
                    ]);
                }
                Some("f") => {
                    let indices: Result<Vec<_>, _> = fields.map(|field| {
                        let index = field.split('/').next().unwrap_or_default()
                            .parse::<isize>().map_err(|_| "invalid OBJ face index")?;
                        if index == 0 {
                            return Err("OBJ face indices cannot be zero");
                        }
                        let resolved = if index < 0 {
                            mesh.vertices.len() as isize + index
                        } else {
                            index - 1
                        };
                        if resolved < 0 || resolved >= mesh.vertices.len() as isize {
                            return Err("OBJ face index out of range");
                        }
                        Ok(resolved as usize)
                    }).collect();
                    let indices = indices?;
                    if indices.len() < 3 {
                        return Err("OBJ face must have at least three vertices".into());
                    }
                    for index in 1..indices.len() - 1 {
                        mesh.faces.push([indices[0], indices[index], indices[index + 1]]);
                    }
                }
                _ => {}
            }
        }
        if mesh.vertices.is_empty() || mesh.faces.is_empty() {
            return Err("OBJ contains no exportable faces".into());
        }
        mesh.normals = vec![[0.0, 0.0, 1.0]; mesh.vertices.len()];
        Ok(mesh)
    }

    fn stl_vertex(vertices: &mut Vec<[f64; 3]>, point: [f64; 3]) -> usize {
            if let Some(index) = vertices.iter().position(|vertex| *vertex == point) {
                index
            } else {
                vertices.push(point);
                vertices.len() - 1
        }
    }
    
    pub fn cube(size: [f64; 3], center: bool) -> Self {
        let [sx, sy, sz] = size;
        let offset = if center { [-size[0]/2.0, -size[1]/2.0, -size[2]/2.0] } else { [0.0, 0.0, 0.0] };
        
        let verts = [
            [0.0, 0.0, 0.0], [sx, 0.0, 0.0], [sx, sy, 0.0], [0.0, sy, 0.0],
            [0.0, 0.0, sz], [sx, 0.0, sz], [sx, sy, sz], [0.0, sy, sz],
        ];
        
        let mut mesh = Mesh::new();
        for v in verts {
            mesh.vertices.push([v[0] + offset[0], v[1] + offset[1], v[2] + offset[2]]);
        }
        
        // 12 triangles (2 per face)
        let faces = [
            [0, 1, 2], [0, 2, 3], // bottom
            [4, 5, 6], [4, 6, 7], // top
            [0, 1, 5], [0, 5, 4], // front
            [2, 3, 7], [2, 7, 6], // back
            [1, 2, 6], [1, 6, 5], // right
            [3, 0, 4], [3, 4, 7], // left
        ];
        
        for f in faces {
            mesh.faces.push(f);
        }
        
        // Simple normals
        mesh.normals = vec![[0.0, 0.0, 1.0]; mesh.vertices.len()];
        
        mesh
    }
    
    pub fn sphere(radius: f64, segments: usize) -> Self {
        let mut mesh = Mesh::new();
        let seg = segments.max(3);

        mesh.vertices.push([0.0, 0.0, radius]);
        for i in 1..seg {
            let lat = std::f64::consts::PI * i as f64 / seg as f64;
            let (sin_lat, cos_lat) = lat.sin_cos();
            for j in 0..seg {
                let (sin_lon, cos_lon) =
                    (2.0 * std::f64::consts::PI * j as f64 / seg as f64).sin_cos();
                mesh.vertices.push([
                    radius * sin_lat * cos_lon,
                    radius * sin_lat * sin_lon,
                    radius * cos_lat,
                ]);
            }
        }

        let south = mesh.vertices.len();
        mesh.vertices.push([0.0, 0.0, -radius]);
        for j in 0..seg {
            let next = (j + 1) % seg;
            mesh.faces.push([0, 1 + next, 1 + j]);
        }
        for ring in 0..seg - 2 {
            let current = 1 + ring * seg;
            let next_ring = current + seg;
            for j in 0..seg {
                let next = (j + 1) % seg;
                mesh.faces.push([current + j, current + next, next_ring + next]);
                mesh.faces.push([current + j, next_ring + next, next_ring + j]);
            }
        }
        let last_ring = 1 + (seg - 2) * seg;
        for j in 0..seg {
            let next = (j + 1) % seg;
            mesh.faces.push([last_ring + j, last_ring + next, south]);
        }
        mesh.normals = vec![[0.0, 0.0, 1.0]; mesh.vertices.len()];
        mesh
    }

    pub fn square(size: [f64; 2], center: bool) -> Self {
        let [sx, sy] = size;
        let [ox, oy] = if center {
            [-sx / 2.0, -sy / 2.0]
        } else {
            [0.0, 0.0]
        };
        Mesh {
            vertices: vec![
                [ox, oy, 0.0],
                [ox + sx, oy, 0.0],
                [ox + sx, oy + sy, 0.0],
                [ox, oy + sy, 0.0],
            ],
            faces: vec![[0, 1, 2], [0, 2, 3]],
            normals: vec![[0.0, 0.0, 1.0]; 4],
        }
    }

    pub fn text(text: &str, size: f64, halign: &str, valign: &str) -> Self {
        let glyph_width = size * 0.6;
        let spacing = size * 0.1;
        let width = text.chars().count() as f64 * glyph_width
            + text.chars().count().saturating_sub(1) as f64 * spacing;
        let offset_x = match halign {
            "center" => -width / 2.0,
            "right" => -width,
            _ => 0.0,
        };
        let offset_y = match valign {
            "center" => -size / 2.0,
            "top" => -size,
            "baseline" | "bottom" => 0.0,
            _ => 0.0,
        };
        let mut mesh = Mesh::new();
        for (index, _) in text.chars().enumerate() {
            let x = offset_x + index as f64 * (glyph_width + spacing);
            let base = mesh.vertices.len();
            mesh.vertices.extend([
                [x, offset_y, 0.0],
                [x + glyph_width, offset_y, 0.0],
                [x + glyph_width, offset_y + size, 0.0],
                [x, offset_y + size, 0.0],
            ]);
            mesh.faces.extend([[base, base + 1, base + 2], [base, base + 2, base + 3]]);
        }
        mesh.normals = vec![[0.0, 0.0, 1.0]; mesh.vertices.len()];
        mesh
    }

    pub fn circle(radius: f64, segments: usize) -> Self {
        let seg = segments.max(3);
        let mut vertices = Vec::with_capacity(seg + 1);
        vertices.push([0.0, 0.0, 0.0]);
        for index in 0..seg {
            let angle = 2.0 * std::f64::consts::PI * index as f64 / seg as f64;
            vertices.push([radius * angle.cos(), radius * angle.sin(), 0.0]);
        }
        let faces = (0..seg)
            .map(|index| [0, index + 1, (index + 1) % seg + 1])
            .collect();
        Mesh {
            vertices,
            faces,
            normals: vec![[0.0, 0.0, 1.0]; seg + 1],
        }
    }

    pub fn convex_hull(points: &[[f64; 3]]) -> Self {
        let mut mesh = Mesh {
            vertices: points.to_vec(),
            faces: Vec::new(),
            normals: vec![[0.0, 0.0, 1.0]; points.len()],
        };
        let epsilon = 1e-9;
        for i in 0..points.len() {
            for j in (i + 1)..points.len() {
                for k in (j + 1)..points.len() {
                    let a = points[i];
                    let b = points[j];
                    let c = points[k];
                    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
                    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
                    let normal = [
                        ab[1] * ac[2] - ab[2] * ac[1],
                        ab[2] * ac[0] - ab[0] * ac[2],
                        ab[0] * ac[1] - ab[1] * ac[0],
                    ];
                    if normal.iter().map(|value| value * value).sum::<f64>() <= epsilon {
                        continue;
                    }
                    let mut positive = false;
                    let mut negative = false;
                    for (index, point) in points.iter().enumerate() {
                        if index == i || index == j || index == k {
                            continue;
                        }
                        let distance = normal[0] * (point[0] - a[0])
                            + normal[1] * (point[1] - a[1])
                            + normal[2] * (point[2] - a[2]);
                        positive |= distance > epsilon;
                        negative |= distance < -epsilon;
                    }
                    if positive && negative {
                        continue;
                    }
                    let face = if positive { [i, k, j] } else { [i, j, k] };
                    if !mesh.faces.contains(&face) {
                        mesh.faces.push(face);
                    }
                }
            }
        }
        mesh
    }

    pub fn minkowski(meshes: &[Mesh]) -> Self {
        let Some(first) = meshes.first() else {
            return Mesh::new();
        };
        let mut points = first.vertices.clone();
        for mesh in meshes.iter().skip(1) {
            points = points.iter().flat_map(|left| {
                mesh.vertices.iter().map(move |right| [
                    left[0] + right[0],
                    left[1] + right[1],
                    left[2] + right[2],
                ])
            }).collect();
        }
        Mesh::convex_hull(&points)
    }
    
    pub fn cylinder(h: f64, r1: f64, r2: f64, segments: usize, center: bool) -> Self {
        let mut mesh = Mesh::new();
        let seg = segments.max(3);
        let z_offset = if center { -0.5 } else { 0.0 };
        
        for i in 0..=1 {
            let z = i as f64 * h + z_offset * h;
            let r = if i == 0 { r1 } else { r2 };
            
            for j in 0..seg {
                let angle = 2.0 * std::f64::consts::PI * j as f64 / seg as f64;
                let x = r * angle.cos();
                let y = r * angle.sin();
                mesh.vertices.push([x, y, z]);
            }
        }
        
        // Side faces
        for j in 0..seg {
            let a = j;
            let b = (j + 1) % seg;
            let c = seg + j;
            let d = seg + (j + 1) % seg;
            mesh.faces.push([a, b, d]);
            mesh.faces.push([a, d, c]);
        }
        
        let bottom_center = mesh.vertices.len();
        mesh.vertices.push([0.0, 0.0, z_offset * h]);
        let top_center = mesh.vertices.len();
        mesh.vertices.push([0.0, 0.0, (1.0 + z_offset) * h]);
        for j in 0..seg {
            let next = (j + 1) % seg;
            mesh.faces.push([bottom_center, next, j]);
            mesh.faces.push([top_center, seg + j, seg + next]);
        }
        mesh.normals = vec![[0.0, 0.0, 1.0]; mesh.vertices.len()];
        mesh
    }
    
    pub fn transform(&mut self, matrix: &nalgebra::Matrix4<f64>) {
        for v in &mut self.vertices {
            let p = nalgebra::Point3::new(v[0], v[1], v[2]);
            let tp = matrix.transform_point(&p);
            v[0] = tp.x;
            v[1] = tp.y;
            v[2] = tp.z;
        }

    }

    pub fn linear_extrude(&self, height: f64, center: bool) -> Mesh {
        let offset_z = if center { -height / 2.0 } else { 0.0 };
        let mut mesh = Mesh::new();
        mesh.vertices.extend(self.vertices.iter().map(|v| [v[0], v[1], offset_z]));
        mesh.vertices.extend(self.vertices.iter().map(|v| [v[0], v[1], offset_z + height]));
        let count = self.vertices.len();
        for face in &self.faces {
            mesh.faces.push([face[2], face[1], face[0]]);
            mesh.faces.push([face[0] + count, face[1] + count, face[2] + count]);
        }

        let mut edges = std::collections::HashMap::<(usize, usize), usize>::new();
        for face in &self.faces {
            for &(a, b) in &[(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
                let edge = if a < b { (a, b) } else { (b, a) };
                *edges.entry(edge).or_default() += 1;
            }
        }
        for ((a, b), count_edges) in edges {
            if count_edges == 1 {
                mesh.faces.push([a, b, b + count]);
                mesh.faces.push([a, b + count, a + count]);
            }
        }
        mesh.normals = vec![[0.0, 0.0, 1.0]; mesh.vertices.len()];
        mesh
    }

    pub fn rotate_extrude(&self, angle: f64, segments: usize) -> Mesh {
        let segments = segments.max(3);
        let sweep = angle.to_radians();
        let mut mesh = Mesh::new();
        for segment in 0..=segments {
            let theta = sweep * segment as f64 / segments as f64;
            let (sin, cos) = theta.sin_cos();
            for vertex in &self.vertices {
                mesh.vertices.push([
                    vertex[0] * cos,
                    vertex[0] * sin,
                    vertex[1],
                ]);
            }
        }
        let count = self.vertices.len();
        for segment in 0..segments {
            let start = segment * count;
            let next = (segment + 1) * count;
            for face in &self.faces {
                for &(a, b) in &[(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
                    mesh.faces.push([start + a, start + b, next + b]);
                    mesh.faces.push([start + a, next + b, next + a]);
                }
            }
        }
        mesh.normals = vec![[0.0, 0.0, 1.0]; mesh.vertices.len()];
        mesh
    }
    
    pub fn union(&self, other: &Mesh) -> Mesh {
        let bounds = |mesh: &Mesh| {
            let mut min = [f64::INFINITY; 3];
            let mut max = [f64::NEG_INFINITY; 3];
            for vertex in &mesh.vertices {
                for axis in 0..3 {
                    min[axis] = min[axis].min(vertex[axis]);
                    max[axis] = max[axis].max(vertex[axis]);
                }
            }
            (min, max)
        };
        let (left_min, left_max) = bounds(self);
        let (right_min, right_max) = bounds(other);
        let disjoint = (0..3).any(|axis| {
            left_max[axis] < right_min[axis] - f64::EPSILON
                || right_max[axis] < left_min[axis] - f64::EPSILON
        });
        if disjoint {
            let mut combined = self.clone();
            let offset = combined.vertices.len();
            combined.vertices.extend_from_slice(&other.vertices);
            combined.faces.extend(other.faces.iter().map(|face| {
                [face[0] + offset, face[1] + offset, face[2] + offset]
            }));
            combined.normals.resize(combined.vertices.len(), [0.0, 0.0, 1.0]);
            return combined;
        }
        // Use manifold for proper boolean operations
        let m1 = Self::mesh_to_manifold(self).unwrap_or_else(|_| Self::mesh_to_manifold_simple(self));
        let m2 = Self::mesh_to_manifold(other).unwrap_or_else(|_| Self::mesh_to_manifold_simple(other));
        let result = m1.union(&m2);
        let mesh = Self::manifold_to_mesh(&result);
        let (result_min, result_max) = bounds(&mesh);
        let preserves_inputs = (0..3).all(|axis| {
            result_min[axis] <= left_min[axis] + f64::EPSILON
                && result_max[axis] >= left_max[axis] - f64::EPSILON
                && result_min[axis] <= right_min[axis] + f64::EPSILON
                && result_max[axis] >= right_max[axis] - f64::EPSILON
        });
        if (!preserves_inputs || mesh.faces.is_empty()) && !self.faces.is_empty() && !other.faces.is_empty() {
            let mut combined = self.clone();
            let offset = combined.vertices.len();
            combined.vertices.extend_from_slice(&other.vertices);
            combined.faces.extend(other.faces.iter().map(|face| {
                [face[0] + offset, face[1] + offset, face[2] + offset]
            }));
            combined.normals.resize(combined.vertices.len(), [0.0, 0.0, 1.0]);
            combined
        } else {
            mesh
        }
    }
    
    pub fn difference(&self, other: &Mesh) -> Mesh {
        let m1 = Self::mesh_to_manifold(self).unwrap_or_else(|_| Self::mesh_to_manifold_simple(self));
        let m2 = Self::mesh_to_manifold(other).unwrap_or_else(|_| Self::mesh_to_manifold_simple(other));
        let result = m1.difference(&m2);
        let mesh = Self::manifold_to_mesh(&result);
        if mesh.faces.is_empty() {
            self.clone()
        } else {
            let base_min = self.vertices.iter().fold([f64::INFINITY; 3], |mut bounds, vertex| {
                for axis in 0..3 {
                    bounds[axis] = bounds[axis].min(vertex[axis]);
                }
                bounds
            });
            let base_max = self.vertices.iter().fold([f64::NEG_INFINITY; 3], |mut bounds, vertex| {
                for axis in 0..3 {
                    bounds[axis] = bounds[axis].max(vertex[axis]);
                }
                bounds
            });
            let result_min = mesh.vertices.iter().fold([f64::INFINITY; 3], |mut bounds, vertex| {
                for axis in 0..3 {
                    bounds[axis] = bounds[axis].min(vertex[axis]);
                }
                bounds
            });
            let result_max = mesh.vertices.iter().fold([f64::NEG_INFINITY; 3], |mut bounds, vertex| {
                for axis in 0..3 {
                    bounds[axis] = bounds[axis].max(vertex[axis]);
                }
                bounds
            });
            if (0..3).any(|axis| result_min[axis] > base_min[axis] + f64::EPSILON
                || result_max[axis] < base_max[axis] - f64::EPSILON)
            {
                self.clone()
            } else {
                mesh
            }
        }
    }

    pub fn intersection(&self, other: &Mesh) -> Mesh {
        let m1 = Self::mesh_to_manifold(self).unwrap_or_else(|_| Self::mesh_to_manifold_simple(self));
        let m2 = Self::mesh_to_manifold(other).unwrap_or_else(|_| Self::mesh_to_manifold_simple(other));
        let result = m1.intersection(&m2);
        Self::manifold_to_mesh(&result)
    }
    
    pub fn manifold_to_mesh(m: &Manifold) -> Mesh {
        let (verts, n_props, tri_indices) = m.to_mesh_f64();
        let mut vertices = Vec::new();
        for i in 0..(verts.len() / n_props) {
            let base = i * n_props;
            if n_props >= 3 {
                vertices.push([verts[base], verts[base + 1], verts[base + 2]]);
            } else {
                vertices.push([verts[base], 0.0, 0.0]);
            }
        }
        let mut faces = Vec::new();
        for i in 0..(tri_indices.len() / 3) {
            let base = i * 3;
            faces.push([tri_indices[base] as usize, tri_indices[base + 1] as usize, tri_indices[base + 2] as usize]);
        }
        let normals = vec![[0.0, 0.0, 1.0]; vertices.len()];
        Mesh { vertices, faces, normals }
    }
    
    fn mesh_to_manifold(m: &Mesh) -> Result<Manifold, manifold3d::CsgError> {
        if m.faces.is_empty() || m.vertices.is_empty() {
            return Manifold::from_mesh_f64(&[], 3, &[]);
        }
        let mut vert_props = Vec::with_capacity(m.vertices.len() * 3);
        for v in &m.vertices {
            vert_props.extend_from_slice(&[v[0], v[1], v[2]]);
        }
        let mut tri_indices: Vec<u64> = Vec::with_capacity(m.faces.len() * 3);
        for f in &m.faces {
            if !m.valid_face(f) {
                continue;
            }
            tri_indices.extend_from_slice(&[f[0] as u64, f[1] as u64, f[2] as u64]);
        }
        Manifold::from_mesh_f64(&vert_props, 3, &tri_indices)
    }
    
    fn mesh_to_manifold_simple(m: &Mesh) -> Manifold {
        Self::mesh_to_manifold(m).unwrap_or_else(|_| Manifold::empty())
    }
    
    pub fn to_obj(&self) -> String {
        let mut out = String::new();
        for v in &self.vertices {
            out.push_str(&format!("v {} {} {}\n", v[0], v[1], v[2]));
        }
        for f in &self.faces {
            out.push_str(&format!("f {} {} {}\n", f[0]+1, f[1]+1, f[2]+1));
        }
        out
    }
    
    pub fn to_stl(&self) -> String {
        let mut out = String::new();
        out.push_str("solid openscad\n");
        for f in &self.faces {
            if !self.valid_face(f) {
                continue;
            }
            let a = self.vertices[f[0]];
            let b = self.vertices[f[1]];
            let c = self.vertices[f[2]];
            
            // Calculate normal
            let ab = [b[0]-a[0], b[1]-a[1], b[2]-a[2]];
            let ac = [c[0]-a[0], c[1]-a[1], c[2]-a[2]];
            let nx = ab[1]*ac[2] - ab[2]*ac[1];
            let ny = ab[2]*ac[0] - ab[0]*ac[2];
            let nz = ab[0]*ac[1] - ab[1]*ac[0];
            let len = (nx*nx + ny*ny + nz*nz).sqrt();
            let normal = if len > 0.0 { [nx/len, ny/len, nz/len] } else { [0.0, 0.0, 1.0] };
            
            out.push_str(&format!("facet normal {} {} {}\n", normal[0], normal[1], normal[2]));
            out.push_str("  outer loop\n");
            out.push_str(&format!("    vertex {} {} {}\n", a[0], a[1], a[2]));
            out.push_str(&format!("    vertex {} {} {}\n", b[0], b[1], b[2]));
            out.push_str(&format!("    vertex {} {} {}\n", c[0], c[1], c[2]));
            out.push_str("  endloop\n");
            out.push_str("endfacet\n");
        }
        out.push_str("endsolid openscad\n");
        out
    }

    pub fn to_binary_stl(&self) -> Vec<u8> {
        let valid_faces = self.faces.iter()
            .filter(|face| self.valid_face(face))
            .collect::<Vec<_>>();
        let mut out = vec![0u8; 80];
        out[..13].copy_from_slice(b"openscad-rust");
        out.extend_from_slice(&(valid_faces.len() as u32).to_le_bytes());
        for face in valid_faces {
            let a = self.vertices[face[0]];
            let b = self.vertices[face[1]];
            let c = self.vertices[face[2]];
            let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
            let normal = [
                ab[1] * ac[2] - ab[2] * ac[1],
                ab[2] * ac[0] - ab[0] * ac[2],
                ab[0] * ac[1] - ab[1] * ac[0],
            ];
            let length = normal.iter().map(|value| value * value).sum::<f64>().sqrt();
            let normal = if length > 0.0 {
                [normal[0] / length, normal[1] / length, normal[2] / length]
            } else {
                [0.0, 0.0, 1.0]
            };
            for value in normal.into_iter()
                .chain(a)
                .chain(b)
                .chain(c)
            {
                out.extend_from_slice(&(value as f32).to_le_bytes());
            }
            out.extend_from_slice(&0u16.to_le_bytes());
        }
        out
    }
}

impl Primitive {
    fn triangulate_polygon(points: &[[f64; 2]], path: &[usize]) -> Vec<[usize; 3]> {
        if path.len() < 3 {
            return Vec::new();
        }
        let area: f64 = path.iter().enumerate().map(|(index, &current)| {
            let next = path[(index + 1) % path.len()];
            points[current][0] * points[next][1] - points[next][0] * points[current][1]
        }).sum();
        let mut remaining = path.to_vec();
        if area < 0.0 {
            remaining.reverse();
        }
        let mut triangles = Vec::with_capacity(path.len() - 2);
        while remaining.len() > 3 {
            let mut clipped = false;
            for index in 0..remaining.len() {
                let previous = remaining[(index + remaining.len() - 1) % remaining.len()];
                let current = remaining[index];
                let next = remaining[(index + 1) % remaining.len()];
                let cross = (points[current][0] - points[previous][0])
                    * (points[next][1] - points[previous][1])
                    - (points[current][1] - points[previous][1])
                    * (points[next][0] - points[previous][0]);
                if cross <= 0.0 {
                    continue;
                }
                let inside = remaining.iter().any(|&candidate| {
                    candidate != previous && candidate != current && candidate != next
                        && Self::point_in_triangle(points[candidate], points[previous], points[current], points[next])
                });
                if !inside {
                    triangles.push([previous, current, next]);
                    remaining.remove(index);
                    clipped = true;
                    break;
                }
            }
            if !clipped {
                return Vec::new();
            }
        }
        triangles.push([remaining[0], remaining[1], remaining[2]]);
        triangles
    }

    fn polygon_area(points: &[[f64; 2]], path: &[usize]) -> f64 {
        path.iter().enumerate().map(|(index, &current)| {
            let next = path[(index + 1) % path.len()];
            points[current][0] * points[next][1] - points[next][0] * points[current][1]
        }).sum::<f64>() / 2.0
    }

    fn point_in_polygon(point: [f64; 2], points: &[[f64; 2]], path: &[usize]) -> bool {
        let mut inside = false;
        for index in 0..path.len() {
            let a = points[path[index]];
            let b = points[path[(index + 1) % path.len()]];
            if (a[1] > point[1]) != (b[1] > point[1])
                && point[0] < (b[0] - a[0]) * (point[1] - a[1]) / (b[1] - a[1]) + a[0]
            {
                inside = !inside;
            }
        }
        inside
    }

    fn segments_cross(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> bool {
        fn orientation(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
            (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
        }
        let ab_c = orientation(a, b, c);
        let ab_d = orientation(a, b, d);
        let cd_a = orientation(c, d, a);
        let cd_b = orientation(c, d, b);
        (ab_c > 0.0 && ab_d < 0.0 || ab_c < 0.0 && ab_d > 0.0)
            && (cd_a > 0.0 && cd_b < 0.0 || cd_a < 0.0 && cd_b > 0.0)
    }

    fn triangulate_polygon_paths(points: &[[f64; 2]], paths: &[Vec<usize>]) -> (Vec<[f64; 2]>, Vec<[usize; 3]>) {
        if paths.len() <= 1 {
            return (points.to_vec(), Self::triangulate_polygon(points, &paths[0]));
        }

        let outer = &paths[0];
        let mut merged = outer.clone();

        for hole in &paths[1..] {
            let hole_index = hole.iter().enumerate()
                .max_by(|(_, &a), (_, &b)| points[a][0].total_cmp(&points[b][0])
                    .then_with(|| points[b][1].total_cmp(&points[a][1])))
                .map(|(index, _)| index)
                .unwrap();
            let hole_vertex = points[hole[hole_index]];
            let outer_index = outer.iter().enumerate()
                .filter(|(_, &index)| {
                    let candidate = points[index];
                    if !Self::point_in_polygon(candidate, points, outer) {
                        return false;
                    }
                    for edge in 0..outer.len() {
                        let a = points[outer[edge]];
                        let b = points[outer[(edge + 1) % outer.len()]];
                        if candidate != a && candidate != b && Self::segments_cross(hole_vertex, candidate, a, b) {
                            return false;
                        }
                    }
                    true
                })
                .min_by(|(_, &a), (_, &b)| {
                    let distance = |index: usize| {
                        let delta = [points[index][0] - hole_vertex[0], points[index][1] - hole_vertex[1]];
                        delta[0] * delta[0] + delta[1] * delta[1]
                    };
                    distance(a).total_cmp(&distance(b))
                })
                .map(|(index, _)| index)
                .unwrap_or_else(|| outer.iter().enumerate()
                    .min_by(|(_, &a), (_, &b)| {
                        let distance = |index: usize| {
                            let delta = [points[index][0] - hole_vertex[0], points[index][1] - hole_vertex[1]];
                            delta[0] * delta[0] + delta[1] * delta[1]
                        };
                        distance(a).total_cmp(&distance(b))
                    }).map(|(index, _)| index).unwrap());

            let position = merged.iter().position(|&index| index == outer[outer_index]).unwrap();
            let mut bridge = Vec::with_capacity(hole.len() + 2);
            bridge.extend(hole[hole_index..].iter().copied());
            bridge.extend(hole[..hole_index].iter().copied());
            bridge.push(hole[hole_index]);
            bridge.push(outer[outer_index]);
            merged.splice(position + 1..position + 1, bridge);
        }

        if Self::polygon_area(points, outer) < 0.0 {
            merged.reverse();
        }
        let merged_points: Vec<[f64; 2]> = merged.iter().map(|&index| points[index]).collect();
        let merged_path: Vec<usize> = (0..merged_points.len()).collect();
        let faces = Self::triangulate_polygon(&merged_points, &merged_path);
        (merged_points, faces)
    }

    fn point_in_triangle(point: [f64; 2], a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> bool {
        let sign = |p1: [f64; 2], p2: [f64; 2], p3: [f64; 2]| {
            (p1[0] - p3[0]) * (p2[1] - p3[1]) - (p2[0] - p3[0]) * (p1[1] - p3[1])
        };
        let d1 = sign(point, a, b);
        let d2 = sign(point, b, c);
        let d3 = sign(point, c, a);
        let has_negative = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
        let has_positive = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
        !(has_negative && has_positive) && d1.abs() > f64::EPSILON
            && d2.abs() > f64::EPSILON && d3.abs() > f64::EPSILON
    }

    fn radial_segments(
        radius: f64,
        fn_: Option<usize>,
        fa: Option<f64>,
        fs: Option<f64>,
    ) -> usize {
        if let Some(segments) = fn_ {
            return segments.max(3);
        }
        let by_angle = fa.filter(|value| *value > 0.0)
            .map(|value| (360.0 / value).ceil() as usize);
        let by_size = fs.filter(|value| *value > 0.0)
            .map(|value| (2.0 * std::f64::consts::PI * radius / value).ceil() as usize);
        by_angle.or(by_size).unwrap_or(16).max(3)
    }

    pub fn to_mesh(&self) -> Mesh {
        match self {
            Primitive::Cube { size, center } => Mesh::cube(*size, *center),
            Primitive::Sphere { r, fn_, fa, fs, .. } => {
                let seg = Self::radial_segments(*r, *fn_, *fa, *fs);
                Mesh::sphere(*r, seg)
            }
            Primitive::Cylinder { h, r1, r2, center, fn_, fa, fs, .. } => {
                let seg = Self::radial_segments((*r1).max(*r2), *fn_, *fa, *fs);
                Mesh::cylinder(*h, *r1, *r2, seg, *center)
            }
            Primitive::Square { size, center } => Mesh::square(*size, *center),
            Primitive::Text { text, size, halign, valign, .. } =>
                Mesh::text(text, *size, halign, valign),
            Primitive::Circle { r, fn_, fa, fs, .. } => {
                let seg = Self::radial_segments(*r, *fn_, *fa, *fs);
                Mesh::circle(*r, seg)
            }
            Primitive::Polyhedron { points, faces } => {
                let mut mesh = Mesh {
                    vertices: points.clone(),
                    faces: Vec::new(),
                    normals: vec![[0.0, 0.0, 1.0]; points.len()],
                };
                for face in faces {
                    if face.len() < 3 {
                        continue;
                    }
                    for index in 1..face.len() - 1 {
                        mesh.faces.push([face[0], face[index], face[index + 1]]);
                    }
                }
                mesh
            }
            Primitive::Polygon { points, paths } => {
                let (polygon_points, faces) = Self::triangulate_polygon_paths(points, paths);
                let mesh = Mesh {
                    vertices: polygon_points.iter().map(|point| [point[0], point[1], 0.0]).collect(),
                    faces,
                    normals: vec![[0.0, 0.0, 1.0]; polygon_points.len()],
                };
                mesh
            }
            _ => Mesh::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CsgBuilder {
    root: Option<CsgNode>,
}

impl CsgBuilder {
    pub fn new() -> Self {
        Self { root: None }
    }

    pub fn from_node(root: CsgNode) -> Self {
        Self { root: Some(root) }
    }
    
    pub fn add_primitive(&mut self, prim: Primitive) -> &mut Self {
        let node = CsgNode::Primitive(prim);
        self.root = Some(match self.root.take() {
            Some(existing) => CsgNode::Boolean {
                op: BooleanOp::Union,
                left: Box::new(existing),
                right: Box::new(node),
            },
            None => node,
        });
        self
    }
    
    pub fn translate(&mut self, v: [f64; 3]) -> &mut Self {
        self.root = self.root.take().map(|c| CsgNode::Transform {
            child: Box::new(c),
            translate: Some(v),
            rotate: None,
            scale: None,
            mirror: None,
            matrix: None,
        });
        self
    }
    
    pub fn rotate(&mut self, v: [f64; 3]) -> &mut Self {
        self.root = self.root.take().map(|c| CsgNode::Transform {
            child: Box::new(c),
            translate: None,
            rotate: Some(v),
            scale: None,
            mirror: None,
            matrix: None,
        });
        self
    }
    
    pub fn scale(&mut self, v: [f64; 3]) -> &mut Self {
        self.root = self.root.take().map(|c| CsgNode::Transform {
            child: Box::new(c),
            translate: None,
            rotate: None,
            scale: Some(v),
            mirror: None,
            matrix: None,
        });
        self
    }
    
    pub fn mirror(&mut self, v: [f64; 3]) -> &mut Self {
        self.root = self.root.take().map(|c| CsgNode::Transform {
            child: Box::new(c),
            translate: None,
            rotate: None,
            scale: None,
            mirror: Some(v),
            matrix: None,
        });
        self
    }
    
    pub fn matrix(&mut self, m: nalgebra::Matrix4<f64>) -> &mut Self {
        self.root = self.root.take().map(|c| CsgNode::Transform {
            child: Box::new(c),
            translate: None,
            rotate: None,
            scale: None,
            mirror: None,
            matrix: Some(m),
        });
        self
    }
    
    pub fn union(&mut self, other: CsgNode) -> &mut Self {
        self.root = Some(match self.root.take() {
            Some(existing) => CsgNode::Boolean {
                op: BooleanOp::Union,
                left: Box::new(existing),
                right: Box::new(other),
            },
            None => other,
        });
        self
    }
    
    pub fn difference(&mut self, other: CsgNode) -> &mut Self {
        self.root = self.root.take().map(|c| CsgNode::Boolean {
            op: BooleanOp::Difference,
            left: Box::new(c),
            right: Box::new(other),
        });
        self
    }
    
    pub fn intersection(&mut self, other: CsgNode) -> &mut Self {
        self.root = self.root.take().map(|c| CsgNode::Boolean {
            op: BooleanOp::Intersection,
            left: Box::new(c),
            right: Box::new(other),
        });
        self
    }
    
    pub fn hull(&mut self, children: Vec<CsgNode>) -> &mut Self {
        let mut all = vec![];
        if let Some(root) = self.root.take() {
            all.push(root);
        }
        all.extend(children);
        self.root = Some(CsgNode::Hull { children: all });
        self
    }
    
    pub fn minkowski(&mut self, children: Vec<CsgNode>) -> &mut Self {
        let mut all = vec![];
        if let Some(root) = self.root.take() {
            all.push(root);
        }
        all.extend(children);
        self.root = Some(CsgNode::Minkowski { children: all });
        self
    }
    
    pub fn color(&mut self, color: [f64; 4]) -> &mut Self {
        self.root = self.root.take().map(|c| CsgNode::Color {
            child: Box::new(c),
            color,
        });
        self
    }
    
    pub fn build(self) -> Option<CsgNode> {
        self.root
    }
}

impl CsgNode {
    pub fn to_mesh(&self) -> Mesh {
        match self {
            CsgNode::Primitive(primitive) => primitive.to_mesh(),
            CsgNode::Transform {
                child,
                translate,
                rotate,
                scale,
                mirror,
                matrix,
            } => {
                let mut mesh = child.to_mesh();
                let mut transform = matrix.unwrap_or_else(Matrix4::identity);
                if let Some(values) = scale {
                    transform *= Matrix4::new_nonuniform_scaling(&nalgebra::Vector3::new(
                        values[0], values[1], values[2],
                    ));
                }
                if let Some(values) = rotate {
                    let rotation = nalgebra::Rotation3::from_euler_angles(
                        values[0].to_radians(),
                        values[1].to_radians(),
                        values[2].to_radians(),
                    );
                    transform *= rotation.to_homogeneous();
                }
                if let Some(values) = mirror {
                    transform *= Matrix4::new_nonuniform_scaling(&nalgebra::Vector3::new(
                        if values[0] == 0.0 { 1.0 } else { -1.0 },
                        if values[1] == 0.0 { 1.0 } else { -1.0 },
                        if values[2] == 0.0 { 1.0 } else { -1.0 },
                    ));
                }
                if let Some(values) = translate {
                    transform *= nalgebra::Translation3::new(
                        values[0], values[1], values[2],
                    ).to_homogeneous();
                }
                mesh.transform(&transform);
                mesh
            }
            CsgNode::Boolean { op, left, right } => {
                let left = left.to_mesh();
                let right = right.to_mesh();
                match op {
                    BooleanOp::Union => left.union(&right),
                    BooleanOp::Difference => left.difference(&right),
                    BooleanOp::Intersection => left.intersection(&right),
                }
            }
            CsgNode::Hull { children } => {
                let points = children.iter()
                    .flat_map(|child| child.to_mesh().vertices)
                    .collect::<Vec<_>>();
                Mesh::convex_hull(&points)
            }
            CsgNode::Minkowski { children } => {
                let meshes = children.iter().map(CsgNode::to_mesh).collect::<Vec<_>>();
                Mesh::minkowski(&meshes)
            }
            CsgNode::Color { child, .. } => child.to_mesh(),
            CsgNode::LinearExtrude { child, height, center } => {
                child.to_mesh().linear_extrude(*height, *center)
            }
            CsgNode::RotateExtrude { child, angle, segments } => {
                child.to_mesh().rotate_extrude(*angle, *segments)
            }
        }
    }
}

impl Default for CsgBuilder {
    fn default() -> Self {
        Self::new()
    }
}