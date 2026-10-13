//! Regression tests for 2D `offset()`, the operation behind
//! `offset(delta=...)` in `.scad` files.
//!
//! The keycap icon (`keycap_logitech.scad`) is a round shape cut with an
//! outline made of `offset(+w/2)` minus `offset(-w/2)`. A folded erosion
//! polygon made the second offset vanish, and a zero-area triangle made
//! Manifold reject the mesh, so the difference returned nothing and the
//! icon was silently missing from the printed part.

use openscad_rust::csg::{Mesh, Primitive};

fn house(s: f64) -> Mesh {
    let points = vec![
        [-0.40 * s, -0.50 * s],
        [0.40 * s, -0.50 * s],
        [0.40 * s, 0.05 * s],
        [0.50 * s, 0.05 * s],
        [0.00, 0.50 * s],
        [-0.50 * s, 0.05 * s],
        [-0.40 * s, 0.05 * s],
    ];
    let path: Vec<usize> = (0..points.len()).collect();
    Primitive::Polygon {
        points,
        paths: vec![path],
    }
    .to_mesh()
}

fn bounds(m: &Mesh) -> (f64, f64, f64, f64) {
    m.vertices
        .iter()
        .fold((f64::MAX, f64::MAX, f64::MIN, f64::MIN), |acc, v| {
            (
                acc.0.min(v[0]),
                acc.1.min(v[1]),
                acc.2.max(v[0]),
                acc.3.max(v[1]),
            )
        })
}

fn area(m: &Mesh) -> f64 {
    m.faces
        .iter()
        .map(|f| {
            let p: Vec<[f64; 3]> = f.iter().map(|&i| m.vertices[i]).collect();
            ((p[1][0] - p[0][0]) * (p[2][1] - p[0][1]) - (p[1][1] - p[0][1]) * (p[2][0] - p[0][0]))
                / 2.0
        })
        .sum::<f64>()
}

#[test]
fn erosion_of_a_sharp_polygon_survives() {
    let m = house(10.0);
    let inner = m.offset(-0.6, 0.0, false);
    assert!(!inner.faces.is_empty(), "erosion produced nothing");
    assert!(area(&inner) > 0.0, "erosion came out with negative area");
    let (_, _, max_x, max_y) = bounds(&inner);
    assert!(
        max_x < 5.0 && max_y < 5.0,
        "erosion did not shrink the shape"
    );
}

#[test]
fn outline_of_a_sharp_shape_is_not_empty() {
    let m = house(10.0);
    let ring = m
        .offset(0.6, 0.0, false)
        .difference(&m.offset(-0.6, 0.0, false));
    assert!(!ring.faces.is_empty(), "the icon outline vanished");
    assert!(area(&ring).abs() > 0.0);
    let (min_x, min_y, max_x, max_y) = bounds(&ring);
    assert!(max_x > 6.0 && max_y > 5.0, "outer mitre spike is missing");
    assert!(min_x < -6.0, "left mitre spike is missing: {min_x}");
    assert!(min_y <= -5.6, "bottom edge moved: {min_y}");
}

#[test]
fn offset_is_deterministic() {
    let m = house(10.0);
    let first = m.offset(-0.6, 0.0, false);
    for _ in 0..8 {
        let again = m.offset(-0.6, 0.0, false);
        assert_eq!(first.vertices, again.vertices);
        assert_eq!(first.faces, again.faces);
    }
}

#[test]
fn triangulation_has_no_zero_area_triangles() {
    // The mitred eaves of the house put three vertices on one line; such a
    // triangle is rejected by Manifold and killed the whole boolean.
    let m = house(10.0);
    for delta in [0.6, -0.6] {
        let offset = m.offset(delta, 0.0, false);
        assert!(!offset.faces.is_empty());
        for f in &offset.faces {
            let p: Vec<[f64; 3]> = f.iter().map(|&i| offset.vertices[i]).collect();
            let a = (p[1][0] - p[0][0]) * (p[2][1] - p[0][1])
                - (p[1][1] - p[0][1]) * (p[2][0] - p[0][0]);
            assert!(a.abs() > 1e-9, "degenerate triangle {f:?}");
        }
    }
}
