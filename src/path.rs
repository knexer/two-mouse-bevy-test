use bevy::prelude::*;
use bevy_xpbd_2d::prelude::*;

pub struct Path {
    pub vertices: Vec<Vec2>,
    pub indices: Vec<[usize; 2]>,
}

impl Path {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn move_to(&mut self, pos: Vec2) {
        self.vertices.push(pos);
    }

    pub fn line_to(&mut self, pos: Vec2) {
        let index = self.vertices.len();
        self.vertices.push(pos);
        self.indices.push([index - 1, index]);
    }

    pub fn arc_to(&mut self, end_pos: Vec2, arc_center: Vec2, num_segments: u32) {
        let start_pos = self.vertices.last().unwrap().clone();
        let radius = (start_pos - arc_center).length();
        let start_angle = f32::atan2(start_pos.y - arc_center.y, start_pos.x - arc_center.x);
        let end_angle = f32::atan2(end_pos.y - arc_center.y, end_pos.x - arc_center.x);
        let sweep_angle = end_angle - start_angle;
        let angle_step = sweep_angle / num_segments as f32;

        for i in 1..=num_segments {
            let angle = start_angle + i as f32 * angle_step;
            let pos = arc_center + Vec2::new(angle.cos(), angle.sin()) * radius;
            self.line_to(pos);
        }
    }

    // TODO write a variant that takes a start angle and end angle.
    // fn arc_to(&mut self, end_pos: Vec2, start_angle: f32, end_angle: f32, num_segments: u32) {

    pub fn close(&mut self) {
        let index = self.vertices.len();
        self.indices.push([index - 1, 0]);
    }

    // Reverse the direction and order of the lines (i.e. reverse winding order).
    pub fn reverse(&mut self) {
        self.indices.reverse();
        for index in self.indices.iter_mut() {
            index.reverse();
        }
    }

    pub fn build_collider(&self) -> Collider {
        let indices_u32 = self
            .indices
            .iter()
            .map(|[a, b]| [*a as u32, *b as u32])
            .collect::<Vec<_>>();
        Collider::polyline(self.vertices.clone(), Some(indices_u32))
    }

    pub fn build_polyline_mesh(&self) -> Mesh {
        let mut mesh = Mesh::new(bevy::render::render_resource::PrimitiveTopology::LineList);
        // LineList docs say:
        // Vertex data is a list of lines. Each pair of vertices composes a new line.
        // Vertices `0 1 2 3` create two lines `0 1` and `2 3`.

        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            self.indices
                .iter()
                .flat_map(|[a, b]| vec![self.vertices[*a], self.vertices[*b]])
                // Must convert to Vec3 because Mesh::ATTRIBUTE_POSITION is Vec3.
                .map(|v| Vec3::new(v.x, v.y, 0.0))
                .collect::<Vec<_>>(),
        );

        return mesh;
    }

    pub fn build_triangle_mesh(&self) -> Mesh {
        // O(n^3) algorithm for triangulating a polygon.
        // https://en.wikipedia.org/wiki/Polygon_triangulation#Ear_clipping_method
        // Could be optimized to O(n^2) by more intelligently searching for ears.
        let mut mesh_indices: Vec<usize> = Vec::new();
        let mut remaining_vertex_indices = self
            .indices
            .iter()
            .map(|[a, _]| *a)
            // .chain(std::iter::once(0))
            .collect::<Vec<_>>();

        while remaining_vertex_indices.len() >= 3 {
            // Find and remove one ear.
            let mut found_ear = false;
            for i in 0..remaining_vertex_indices.len() {
                let prev_i =
                    (i + remaining_vertex_indices.len() - 1) % remaining_vertex_indices.len();
                let next_i = (i + 1) % remaining_vertex_indices.len();
                // O(n). Could be cached but would have to invalidate it when removing adjacent vertices.
                if is_ear(
                    self,
                    remaining_vertex_indices[i],
                    remaining_vertex_indices[prev_i],
                    remaining_vertex_indices[next_i],
                ) {
                    // Emit a triangle: (vertex.prev, ear, vertex.next)
                    let a = remaining_vertex_indices[prev_i];
                    let b = remaining_vertex_indices[i];
                    let c = remaining_vertex_indices[next_i];
                    mesh_indices.push(a);
                    mesh_indices.push(b);
                    mesh_indices.push(c);
                    // Delete it from the vertex list.
                    remaining_vertex_indices.remove(i);
                    found_ear = true;
                    break;
                }
            }
            assert!(found_ear);
        }

        let mut mesh = Mesh::new(bevy::render::render_resource::PrimitiveTopology::TriangleList);
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            mesh_indices
                .iter()
                .map(|i| self.vertices[*i])
                // Must convert to Vec3 because Mesh::ATTRIBUTE_POSITION is Vec3.
                .map(|v| Vec3::new(v.x, v.y, 0.0))
                .collect::<Vec<_>>(),
        );

        mesh
    }
}

fn is_ear(path: &Path, ear: usize, prev: usize, next: usize) -> bool {
    // Check if any vertices in path are contained in the triangle (prev, ear, next).
    // If so, return false.
    // Otherwise, return true.

    let ear_pos = path.vertices[ear];
    let prev_pos = path.vertices[prev];
    let next_pos = path.vertices[next];

    println!("Checking ear: {} {} {}", prev, ear, next);
    println!("{} {} {}", prev_pos, ear_pos, next_pos);

    for i in 0..path.vertices.len() {
        if i == ear || i == prev || i == next {
            continue;
        }

        let pos = path.vertices[i];
        println!("{} {}", i, pos);
        if is_point_in_triangle(pos, prev_pos, ear_pos, next_pos) {
            println!("Point {} is in triangle", i);
            return false;
        }
    }

    return true;
}

// fn is_point_in_triangle(point: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
//     // https://stackoverflow.com/a/2049593/5434860
//     let area = 0.5 * (-b.y * c.x + a.y * (-b.x + c.x) + a.x * (b.y - c.y) + b.x * c.y);
//     let s = 1.0 / (2.0 * area)
//         * (a.y * c.x - a.x * c.y + (c.y - a.y) * point.x + (a.x - c.x) * point.y);
//     let t = 1.0 / (2.0 * area)
//         * (a.x * b.y - a.y * b.x + (a.y - b.y) * point.x + (b.x - a.x) * point.y);

//     return s > 0.0 && t > 0.0 && 1.0 - s - t > 0.0;
// }

fn sign(p1: Vec2, p2: Vec2, p3: Vec2) -> f32 {
    return (p1.x - p3.x) * (p2.y - p3.y) - (p2.x - p3.x) * (p1.y - p3.y);
}

fn is_point_in_triangle(point: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
    let d1 = sign(point, a, b);
    let d2 = sign(point, b, c);
    let d3 = sign(point, c, a);

    println!("{} {} {}", d1, d2, d3);

    let has_neg = (d1 < 0.0) || (d2 < 0.0) || (d3 < 0.0);
    let has_pos = (d1 > 0.0) || (d2 > 0.0) || (d3 > 0.0);
    let has_eq = (d1 == 0.0) || (d2 == 0.0) || (d3 == 0.0);

    println!("{} {} {}", has_neg, has_pos, has_eq);
    println!("Returning {}", !has_eq && !(has_neg && has_pos));

    return has_eq || !(has_neg && has_pos);
}
