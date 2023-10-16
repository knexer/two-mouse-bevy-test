use bevy::prelude::*;
use bevy_xpbd_2d::prelude::*;

pub struct Path {
    pub vertices: Vec<Vec2>,
    pub indices: Vec<[usize; 2]>,
}

pub enum WindDirection {
    Clockwise,
    CounterClockwise,
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

    pub fn arc_to(
        &mut self,
        end_pos: Vec2,
        arc_center: Vec2,
        num_segments: u32,
        direction: WindDirection,
    ) {
        let start_pos = self.vertices.last().unwrap().clone();
        let radius = (start_pos - arc_center).length();
        let start_angle = f32::atan2(start_pos.y - arc_center.y, start_pos.x - arc_center.x);
        let end_angle = f32::atan2(end_pos.y - arc_center.y, end_pos.x - arc_center.x);
        let sweep_angle = match direction {
            WindDirection::Clockwise => end_angle - start_angle,
            WindDirection::CounterClockwise => start_angle - end_angle,
        };
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

    pub fn reverse_winding_order(&mut self) {
        self.indices.reverse();
        for index in self.indices.iter_mut() {
            index.reverse();
        }
    }

    pub fn build_collider(&self) -> Collider {
        let triangles_u32 = self
            .triangulate()
            .iter()
            .map(|[a, b, c]| [*a as u32, *b as u32, *c as u32])
            .collect::<Vec<_>>();
        Collider::trimesh(self.vertices.clone(), triangles_u32)
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
        let triangles = self.triangulate();

        let mut mesh = Mesh::new(bevy::render::render_resource::PrimitiveTopology::TriangleList);
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            triangles
                .iter()
                .flatten()
                .map(|i| self.vertices[*i])
                // Must convert to Vec3 because Mesh::ATTRIBUTE_POSITION is Vec3.
                .map(|v| Vec3::new(v.x, v.y, 0.0))
                .collect::<Vec<_>>(),
        );

        mesh
    }

    fn triangulate(&self) -> Vec<[usize; 3]> {
        // O(n^3) algorithm for triangulating a polygon.
        // https://en.wikipedia.org/wiki/Polygon_triangulation#Ear_clipping_method
        // Could be optimized to O(n^2) by more intelligently searching for ears.
        let mut triangles: Vec<[usize; 3]> = Vec::new();
        let mut remaining_vertex_indices = self
            .indices
            .iter()
            .map(|[a, _]| *a)
            // .chain(std::iter::once(0))
            .collect::<Vec<_>>();

        while remaining_vertex_indices.len() >= 3 {
            // Find and remove one ear.
            let mut found_ear = false;
            for index_index in 0..remaining_vertex_indices.len() {
                let prev_index_index = (index_index + remaining_vertex_indices.len() - 1)
                    % remaining_vertex_indices.len();
                let next_index_index = (index_index + 1) % remaining_vertex_indices.len();
                // O(n). Could be cached but would have to invalidate it when removing adjacent vertices.
                if is_ear(
                    self,
                    remaining_vertex_indices[index_index],
                    remaining_vertex_indices[prev_index_index],
                    remaining_vertex_indices[next_index_index],
                ) {
                    // Emit a triangle: (vertex.prev, ear, vertex.next)
                    triangles.push([
                        remaining_vertex_indices[prev_index_index],
                        remaining_vertex_indices[index_index],
                        remaining_vertex_indices[next_index_index],
                    ]);
                    // Delete ear from the vertex list, leaving us with a smaller polygon.
                    remaining_vertex_indices.remove(index_index);
                    found_ear = true;
                    break;
                }
            }
            assert!(
                found_ear,
                "Failed to find an ear, is the polygon self-intersecting?"
            );
        }

        return triangles;
    }
}

fn is_ear(path: &Path, ear: usize, prev: usize, next: usize) -> bool {
    let ear_pos = path.vertices[ear];
    let prev_pos = path.vertices[prev];
    let next_pos = path.vertices[next];

    // Verify that the triangle is counter-clockwise oriented (i.e. is inside the polygon, a 'front face').
    if sign(prev_pos, ear_pos, next_pos) <= 0.0 {
        return false;
    }

    // Verify there are no other vertices inside the triangle.
    for i in 0..path.vertices.len() {
        if i == ear || i == prev || i == next {
            continue;
        }

        let pos = path.vertices[i];
        if is_point_in_triangle(pos, prev_pos, ear_pos, next_pos) {
            return false;
        }
    }

    return true;
}

fn sign(p1: Vec2, p2: Vec2, p3: Vec2) -> f32 {
    return (p1.x - p3.x) * (p2.y - p3.y) - (p2.x - p3.x) * (p1.y - p3.y);
}

fn is_point_in_triangle(point: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
    let d1 = sign(point, a, b);
    let d2 = sign(point, b, c);
    let d3 = sign(point, c, a);

    let has_neg = (d1 < 0.0) || (d2 < 0.0) || (d3 < 0.0);
    let has_pos = (d1 > 0.0) || (d2 > 0.0) || (d3 > 0.0);
    let all_eq = (d1 == 0.0) && (d2 == 0.0) && (d3 == 0.0);

    // Point is definitely outside the triangle if it's on negative side of one edge and positive
    // side of another edge.
    if has_neg && has_pos {
        return false;
    }

    // Point is definitely inside the triangle there's at least one edge it isn't collinear with.
    if !all_eq {
        return true;
    }

    // If we're here, we know the point is collinear with all three edges.
    // For now, just return false.
    return false;
}
