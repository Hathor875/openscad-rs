// Simple CSG module using mesh representation
// Uses nalgebra for math, outputs to OBJ/STL

use nalgebra::Matrix4;
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
        
        // Top and bottom caps (simplified)
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
    
    pub fn union(&self, other: &Mesh) -> Mesh {
        // Simple concatenation (no actual boolean union)
        let mut result = self.clone();
        let offset = result.vertices.len();
        result.vertices.extend(other.vertices.iter().cloned());
        for f in &other.faces {
            result.faces.push([f[0] + offset, f[1] + offset, f[2] + offset]);
        }
        result.normals.extend(other.normals.iter().cloned());
        result
    }
    
    pub fn difference(&self, _other: &Mesh) -> Mesh {
        // Placeholder - just return self
        self.clone()
    }
    
    pub fn intersection(&self, _other: &Mesh) -> Mesh {
        // Placeholder
        self.clone()
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
    pub fn to_mesh(&self) -> Mesh {
        match self {
            Primitive::Cube { size, center } => Mesh::cube(*size, *center),
            Primitive::Sphere { r, fn_, .. } => {
                let seg = fn_.unwrap_or(16).max(3);
                Mesh::sphere(*r, seg)
            }
            Primitive::Cylinder { h, r1, r2, center, fn_, .. } => {
                let seg = fn_.unwrap_or(16).max(3);
                Mesh::cylinder(*h, *r1, *r2, seg, *center)
            }
            Primitive::Square { size, center } => {
                Mesh::cube([size[0], size[1], 0.001], *center)
            }
            Primitive::Circle { r, fn_, .. } => {
                let seg = fn_.unwrap_or(16).max(3);
                Mesh::sphere(*r, seg)
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
    
    pub fn add_primitive(&mut self, prim: Primitive) -> &mut Self {
        self.root = Some(CsgNode::Primitive(prim));
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
        self.root = self.root.take().map(|c| CsgNode::Boolean {
            op: BooleanOp::Union,
            left: Box::new(c),
            right: Box::new(other),
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

impl Default for CsgBuilder {
    fn default() -> Self {
        Self::new()
    }
}