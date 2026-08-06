use glint::{Window, Mesh, Shader};
use tobj;

pub struct ColorGrid {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<f32>,
}

pub fn load_obj(obj_file: &str) -> Vec<tobj::Model> {
    let obj_data = tobj::load_obj(
        obj_file,
        &tobj::LoadOptions {
            triangulate: true,
            ..Default::default()
        },
    );

    let (models, _) = obj_data.expect("Failed to load OBJ file");

    models
}

fn get_cam_scale(size: (i32, i32)) -> (f32, f32) {
    let aspect = size.0 as f32 / size.1 as f32;

    if aspect >= 1.0 {
        // Wide window
        (aspect, 1.0)
    } else {
        // Tall window
        (1.0,  1.0 / aspect)
    }
}

pub fn render(app: &mut Window, shader: &Shader, mesh: &Mesh, grid: ColorGrid) {
    if grid.cells.len() != grid.width * grid.height * 3 {
        println!("Grid length was not the proper length! Cells are dropped!");
    }

    app.clear_with_color(0.1, 0.1, 0.1);

    let cam_scale = get_cam_scale((app.width, app.height));
    shader.set_vec2("cam_scale", [cam_scale.0 * 10.0, cam_scale.1 * 10.0]);

    //go through each of the grid cells, render a quad in the correct position with a color
    for (i, col) in grid.cells.chunks(3).enumerate() {
        //calculate position of quad
        let x = i % grid.width;
        let y = (i - x) / grid.width;

        shader.set_vec2("position", [(x as f32) * 1.1, (y as f32) * -1.1]);
        shader.set_vec2("scale", [1.0, 1.0]);
        shader.set_vec3("color", [col[0], col[1], col[2]]);

        mesh.draw();
    }

    app.swap_buffers();
}