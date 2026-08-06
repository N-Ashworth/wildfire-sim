use glint::{Window, Mesh, Shader};

mod renderer;
mod generator;

use renderer::{ColorGrid, load_obj};
use generator::{cell_grid_to_colors, gen_grid};

fn main() {
    // Rendering and window stuff
    let mut app = Window::new(960, 540, "Cellular Automata Test");

    let shader = Shader::new(r"src\shader.vert", r"src\shader.frag");
    shader.bind();

    let quad_obj = load_obj("quad.obj")[0].mesh.clone();
    let quad = Mesh::new(quad_obj.positions, quad_obj.indices, 3);

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

        // ----- RENDERING -----
        let grid = ColorGrid {
            width: grid.width,
            height: grid.height,
            cells: cell_grid_to_colors(&grid),
        };

        renderer::render(&mut app, &shader, &quad, grid);

        last_time = current_time;
    }
}