use glint::{Window, Mesh, Shader};
use glfw::MouseButton;
mod renderer;
mod generator;
pub mod noise;

use renderer::{ColorGrid, load_obj};
use generator::{cell_grid_to_colors, gen_grid};

fn screen_to_world(screen_delta: (f32, f32), window_size: (i32, i32)) -> (f32, f32) {
    let cam_scale = renderer::get_cam_scale(window_size);

    (
        screen_delta.0 * cam_scale.0 * 1.1 / window_size.0 as f32,
        -screen_delta.1 * cam_scale.1 * 2.0 / window_size.1 as f32,
    )
}

fn main() {
    // Rendering and window stuff
    let mut app = Window::new(960, 540, "Cellular Automata Test");

    let shader = Shader::new(r"src\shader.vert", r"src\shader.frag");
    shader.bind();

    let quad_obj = load_obj("quad.obj")[0].mesh.clone();
    let quad = Mesh::new(quad_obj.positions, quad_obj.indices, 3);

    let mut cam_pos = (0.0, 0.0);

    //Frames and deltatime
    let mut last_time = 0.0;

    let mut fps_timer = 0.0;
    let mut frames = 0;

    //Global variables
    let mut grid = gen_grid(20, 20);

    //Main loop
    while app.running() {
        //POLL EVENTS
        app.poll_events();

        // ----- DT CALCULATION -----
        let current_time = app.time();
        let delta_time = current_time - last_time;

        frames += 1;
        fps_timer += delta_time;

        if fps_timer > 1.0 {
            println!("{}", frames);

            fps_timer = 0.0;
            frames = 0;
        }

        //CAMERA DRAG
        if app.input.is_mouse_held(MouseButton::Button1) {
            let cam_mvmt = screen_to_world((app.input.mouse_delta.0 as f32, app.input.mouse_delta.1 as f32), (app.width, app.height));
            cam_pos.0 -= cam_mvmt.0;
            cam_pos.1 -= cam_mvmt.1;
        }

        // ----- RENDERING -----
        let grid = ColorGrid {
            width: grid.width,
            height: grid.height,
            cells: cell_grid_to_colors(&grid),
        };

        renderer::render(&mut app, &shader, &quad, grid, cam_pos);

        last_time = current_time;
    }
}