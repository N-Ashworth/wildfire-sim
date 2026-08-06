use glint::{Window, Mesh, Shader};
use glfw::MouseButton;
mod renderer;
mod generator;
pub mod noise;

use renderer::{ColorGrid, load_obj};
use generator::{cell_grid_to_colors, gen_grid};

fn screen_delta_to_world(
    delta: (f32, f32),
    window_size: (i32, i32),
    zoom: f32,
) -> (f32, f32) {
    let cam_scale = renderer::get_cam_scale(window_size);

    (
        delta.0 * cam_scale.0 * 2.0 / window_size.0 as f32 / zoom,
        -delta.1 * cam_scale.1 * 2.0 / window_size.1 as f32 / zoom,
    )
}

fn screen_to_world(
    screen_pos: (f32, f32),
    window_size: (i32, i32),
    cam_pos: (f32, f32),
    zoom: f32,
) -> (f32, f32) {
    let cam_scale = renderer::get_cam_scale(window_size);

    let clip_x = screen_pos.0 / window_size.0 as f32 * 2.0 - 1.0;
    let clip_y = 1.0 - screen_pos.1 / window_size.1 as f32 * 2.0;

    (
        cam_pos.0 + clip_x * cam_scale.0 * 10.0 / zoom,
        cam_pos.1 + clip_y * cam_scale.1 * 10.0 / zoom,
    )
}

fn main() {
    // Rendering and window stuff
    let mut app = Window::new(960, 540, "Wildfire Simulation");

    let shader = Shader::new(r"src\shader.vert", r"src\shader.frag");
    shader.bind();

    let quad_obj = load_obj("quad.obj")[0].mesh.clone();
    let quad = Mesh::new(quad_obj.positions, quad_obj.indices, 3);

    let mut cam_pos = (0.0, 0.0);
    let mut zoom = 1.0;

    //Frames and deltatime
    let mut last_time = 0.0;

    let mut fps_timer = 0.0;
    let mut frames = 0;

    //Global variables
    let mut grid = gen_grid(50, 50);

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
            let cam_mvmt = screen_delta_to_world((app.input.mouse_delta.0 as f32, app.input.mouse_delta.1 as f32), (app.width, app.height), zoom / 10.0);
            cam_pos.0 -= cam_mvmt.0;
            cam_pos.1 -= cam_mvmt.1;
        }

        //CAMERA ZOOM
        let scroll = app.input.scroll;

        if scroll != 0.0 {
            let scr_m = app.input.mouse_position;

            // World position under mouse BEFORE zoom
            let before = screen_to_world(
                (scr_m.0 as f32, scr_m.1 as f32),
                (app.width, app.height),
                cam_pos,
                zoom,
            );

            let zoom_f = (1.1f64).powf(scroll);
            zoom *= zoom_f as f32;

            // World position under mouse AFTER zoom
            let after = screen_to_world(
                (scr_m.0 as f32, scr_m.1 as f32),
                (app.width, app.height),
                cam_pos,
                zoom,
            );

            // Shift camera so they line up again
            cam_pos.0 += before.0 - after.0;
            cam_pos.1 += before.1 - after.1;
        }
        //UPDATE GRID
        grid = grid.update(delta_time);

        // ----- RENDERING -----
        let grid = ColorGrid {
            width: grid.width,
            height: grid.height,
            cells: cell_grid_to_colors(&grid),
        };

        renderer::render(&mut app, &shader, &quad, grid, cam_pos, zoom as f32);

        last_time = current_time;
    }
}