use rand::Rng;
use crate::noise::{fbm};

enum Cell {
    Land,
    Water,
}

fn cell_to_color(cell: &Cell) -> [f32; 3] {
    match cell {
        Cell::Land => [0.0, 1.0, 0.0],
        Cell::Water => [0.0, 0.0, 1.0],
    }
}

pub struct CellGrid {
    pub width: usize,
    pub height: usize,
    cells: Vec<Cell>,
}

pub fn cell_grid_to_colors(grid: &CellGrid) -> Vec<f32> {
    let mut colors: Vec<f32> = vec![];
    for c in &grid.cells {
        colors.extend(Vec::from(cell_to_color(c)));
    }
    colors
}

pub fn gen_grid(width: usize, height: usize) -> CellGrid {

    let mut cells: Vec<Cell> = vec![];

    for i in 0..(width * height) {
        let height = fbm((i as f32 % width as f32, (i as i32 / width as i32) as f32), 0.5);

        if height < 0.2 {
            cells.push(Cell::Water);
        } else {
            cells.push(Cell::Land);
        }
    }

    CellGrid {
        width,
        height,
        cells,
    }
}