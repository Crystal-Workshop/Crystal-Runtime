use std::collections::HashMap;

use anyhow::{anyhow, Context, Result};
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// GPU ready mesh buffers produced from an OBJ file.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ObjMesh {
    pub vertices: Vec<f32>,
    pub indices: Vec<u32>,
}

/// Parses an OBJ file from memory and returns interleaved vertex/index arrays.
///
/// Vertices are laid out as `position.xyz` followed by `normal.xyz`.
pub fn load_obj_from_str(data: &str) -> Result<ObjMesh> {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut faces: Vec<[FaceIndex; 3]> = Vec::new();

    for (line_no, line) in data.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        let Some(tag) = parts.next() else {
            continue;
        };
        match tag {
            "v" => positions.push(
                parse_vec3(parts)
                    .with_context(|| format!("invalid vertex on line {}", line_no + 1))?,
            ),
            "vn" => normals.push(
                parse_vec3(parts)
                    .with_context(|| format!("invalid normal on line {}", line_no + 1))?,
            ),
            "f" => {
                let polygon = parse_face(parts)
                    .with_context(|| format!("invalid face on line {}", line_no + 1))?;
                triangulate_face(&polygon, &mut faces);
            }
            _ => {}
        }
    }

    if positions.is_empty() {
        return Err(anyhow!("OBJ file does not define any vertices"));
    }

    let mut mesh = build_mesh(&positions, &normals, &faces)?;
    if needs_normals(&mesh.vertices) {
        compute_normals(&mut mesh);
    }
    Ok(mesh)
}

fn parse_vec3<'a>(mut parts: impl Iterator<Item = &'a str>) -> Result<Vec3> {
    let x = parts
        .next()
        .ok_or_else(|| anyhow!("missing vector component"))?
        .parse::<f32>()?;
    let y = parts
        .next()
        .ok_or_else(|| anyhow!("missing vector component"))?
        .parse::<f32>()?;
    let z = parts
        .next()
        .ok_or_else(|| anyhow!("missing vector component"))?
        .parse::<f32>()?;
    Ok(Vec3::new(x, y, z))
}

fn parse_face<'a>(parts: impl Iterator<Item = &'a str>) -> Result<Vec<FaceIndex>> {
    let mut indices = Vec::new();
    for part in parts {
        let mut segments = part.split('/');
        let vi = segments
            .next()
            .ok_or_else(|| anyhow!("missing vertex index"))?
            .parse::<i32>()?;
        let vt = segments
            .next()
            .map(|s| {
                if s.is_empty() {
                    0
                } else {
                    s.parse::<i32>().unwrap_or(0)
                }
            })
            .unwrap_or(0);
        let vn = segments
            .next()
            .map(|s| {
                if s.is_empty() {
                    0
                } else {
                    s.parse::<i32>().unwrap_or(0)
                }
            })
            .unwrap_or(0);
        indices.push(FaceIndex { v: vi, vn, _vt: vt });
    }
    if indices.len() < 3 {
        return Err(anyhow!("faces must reference at least 3 vertices"));
    }
    Ok(indices)
}

fn triangulate_face(polygon: &[FaceIndex], faces: &mut Vec<[FaceIndex; 3]>) {
    if polygon.len() < 3 {
        return;
    }
    for i in 1..(polygon.len() - 1) {
        faces.push([polygon[0], polygon[i], polygon[i + 1]]);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Key {
    position: usize,
    normal: Option<usize>,
}

#[derive(Debug, Clone, Copy)]
struct FaceIndex {
    v: i32,
    _vt: i32,
    vn: i32,
}

fn build_mesh(positions: &[Vec3], normals: &[Vec3], faces: &[[FaceIndex; 3]]) -> Result<ObjMesh> {
    let mut lookup: HashMap<Key, u32> = HashMap::new();
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for face in faces {
        for idx in face {
            let pos_index =
                fix_index(idx.v, positions.len()).ok_or_else(|| anyhow!("invalid vertex index"))?;
            let normal_index = fix_index(idx.vn, normals.len());
            let key = Key {
                position: pos_index,
                normal: normal_index,
            };
            let next_index = (vertices.len() / 6) as u32;
            let entry = lookup.entry(key).or_insert_with(|| {
                let position = positions[pos_index];
                vertices.extend_from_slice(&[position.x, position.y, position.z]);
                let normal = normal_index.map(|i| normals[i]).unwrap_or(Vec3::ZERO);
                vertices.extend_from_slice(&[normal.x, normal.y, normal.z]);
                next_index
            });
            indices.push(*entry);
        }
    }

    Ok(ObjMesh { vertices, indices })
}

fn fix_index(index: i32, len: usize) -> Option<usize> {
    if index > 0 {
        let zero_based = index as usize - 1;
        (zero_based < len).then_some(zero_based)
    } else if index < 0 {
        let abs = (-index) as usize;
        (abs <= len).then_some(len - abs)
    } else {
        None
    }
}

fn needs_normals(vertices: &[f32]) -> bool {
    vertices
        .chunks_exact(6)
        .any(|chunk| chunk[3] == 0.0 && chunk[4] == 0.0 && chunk[5] == 0.0)
}

fn compute_normals(mesh: &mut ObjMesh) {
    let vertex_count = mesh.vertices.len() / 6;
    let mut accum = vec![Vec3::ZERO; vertex_count];

    for triangle in mesh.indices.chunks_exact(3) {
        let i0 = triangle[0] as usize;
        let i1 = triangle[1] as usize;
        let i2 = triangle[2] as usize;
        let p0 = Vec3::from_slice(&mesh.vertices[i0 * 6..i0 * 6 + 3]);
        let p1 = Vec3::from_slice(&mesh.vertices[i1 * 6..i1 * 6 + 3]);
        let p2 = Vec3::from_slice(&mesh.vertices[i2 * 6..i2 * 6 + 3]);
        let normal = (p1 - p0).cross(p2 - p0);
        if normal.length_squared() > f32::EPSILON {
            let normal = normal.normalize();
            accum[i0] += normal;
            accum[i1] += normal;
            accum[i2] += normal;
        }
    }

    for (i, normal) in accum.into_iter().enumerate() {
        let normal = normal.normalize_or_zero();
        mesh.vertices[i * 6 + 3] = normal.x;
        mesh.vertices[i * 6 + 4] = normal.y;
        mesh.vertices[i * 6 + 5] = normal.z;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_triangle() {
        let obj = "\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";
        let mesh = load_obj_from_str(obj).unwrap();
        assert_eq!(mesh.indices, vec![0, 1, 2]);
        assert_eq!(mesh.vertices.len(), 18);
    }

    #[test]
    fn computes_missing_normals() {
        let obj = "\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";
        let mesh = load_obj_from_str(obj).unwrap();
        for chunk in mesh.vertices.chunks_exact(6) {
            let normal = Vec3::new(chunk[3], chunk[4], chunk[5]);
            assert!((normal.length() - 1.0).abs() < 1e-5);
        }
    }
}
