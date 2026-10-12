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
        
        // Generate icosphere or UV sphere
        for i in 0..=seg {
            let lat = std::f64::consts::PI * i as f64 / seg as f64;
            let sin_lat = lat.sin();
            let cos_lat = lat.cos();
            
            for j in 0..seg {
                let lon = 2.0 * std::f64::consts::PI * j as f64 / seg as f64;
                let sin_lon = lon.sin();
                let cos_lon = lon.cos();
                
                let x = radius * sin_lat * cos_lon;
                let y = radius * sin_lat * sin_lon;
                let z = radius * cos_lat;
                
                mesh.vertices.push([x, y, z]);
            }
        }
        
        // Generate faces (simplified - just pole caps)
        // This is a very simplified version
        for i in 0..seg-1 {
            for j in 0..seg {
                let a = i * seg + j;
                let b = i * seg + (j + 1) % seg;
                let c = (i + 1) * seg + j;
                let d = (i + 1) * seg + (j + 1) % seg;
                
                mesh.faces.push([a, b, d]);
                mesh.faces.push([a, d, c]);
            }
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
        // Use manifold for proper boolean operations
        let m1 = Self::mesh_to_manifold(self).unwrap_or_else(|_| Self::mesh_to_manifold_simple(self));
        let m2 = Self::mesh_to_manifold(other).unwrap_or_else(|_| Self::mesh_to_manifold_simple(other));
        let result = m1.union(&m2);
        let mesh = Self::manifold_to_mesh(&result);
        if mesh.faces.is_empty() && !self.faces.is_empty() && !other.faces.is_empty() {
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
        Self::manifold_to_mesh(&result)
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
}

impl Primitive {
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
                let mut mesh = Mesh {
                    vertices: points.iter().map(|point| [point[0], point[1], 0.0]).collect(),
                    faces: Vec::new(),
                    normals: vec![[0.0, 0.0, 1.0]; points.len()],
                };
                for path in paths {
                    if path.len() < 3 {
                        continue;
                    }
                    for index in 1..path.len() - 1 {
                        mesh.faces.push([path[0], path[index], path[index + 1]]);
                    }
                }
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
            CsgNode::Hull { children } | CsgNode::Minkowski { children } => {
                children.first().map(CsgNode::to_mesh).unwrap_or_else(Mesh::new)
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