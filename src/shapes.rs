#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 3],
}

// put this in a struct VVVV
// implement_vertex!(Vertex, position, color);