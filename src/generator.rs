use rand::Rng;

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
    let mut rng = rand::rng();

    let mut cells: Vec<Cell> = vec![];

    for i in 0..(width * height) {
        let height = rng.random_range(0.0..10.0);

        if height < 4.0 {
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