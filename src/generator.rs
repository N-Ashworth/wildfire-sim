use crate::noise::{fbm};
use rand::Rng;

// ----- GLOBAL SIMULATION PARAMETERS -----

/*  HOW THE SIMULATION WORKS
This simulation is a Cellular Automata. That means it's a grid of cells (pixels basically)
each with their own state (Burned land, land with strong wind, fire, water, etc.).
Every frame, the simulation updates with each cell getting a new state based on the states of its neighbors.

The rules for the update function are

1. Each cell has a "oxygen" value and a "fuel" value.
Cells that are not on fire raise their oxygen, cells on fire lower the oxygen.
Cells on fire also lower the fuel value, but slower (15x) and it doesn't replenish.

The chance of a fire to go out is proportional to its oxygen X its fuel.

2. If more of a cell's neighbors are on fire, the higher chance it goes on fire as well.
This one is self-explanatory.

3. If a cell is on fire, it spreads its fire in the direction of the wind.
This one is also self-explanatory.

note: these are all f32s (floating-point numbers, ones with a decimal)
so if you put 5.0 it will work fine but if you put, say, 5, the compiler
will think it's an int (integer, no decimal) and throw an error.   */

//Fire spread and Extinguish
const SPREAD_FACTOR: f32 = 0.2;

const NEIGHBOR_SPREAD_FACTOR: f32 = 0.5;
const WIND_SPREAD_FACTOR: f32 = 0.25;

const EXTINGUISH_FACTOR: f32 = 0.0;

//Oxygen and Fuel
const OXYGEN_BURN_FACTOR: f32 = 3.0;
const OXYGEN_REGROW_FACTOR: f32 = 0.003;

const FUEL_BURN_FACTOR: f32 = 0.2;
#[derive(Clone)]
enum Cell {
    Land {
        fire: bool,
        o2: f32,
        wind: (f32, f32),
        fuel: f32,
    },
    Water,
}

fn lerp(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn cell_to_color(cell: &Cell) -> [f32; 3] {
    match *cell {
        Cell::Water => [0.08, 0.25, 0.65],

        Cell::Land {
            fire,
            o2,
            wind: _,
            fuel,
        } => {
            if fire {
                // Burning: ember -> orange -> yellow -> white
                let t = o2.clamp(0.0, 1.0);

                let ember = [0.60, 0.08, 0.03];
                let orange = [1.00, 0.45, 0.05];
                let yellow = [1.00, 0.90, 0.25];
                let white = [1.00, 0.98, 0.85];

                if t < 0.33 {
                    lerp(ember, orange, t / 0.33)
                } else if t < 0.66 {
                    lerp(orange, yellow, (t - 0.33) / 0.33)
                } else {
                    lerp(yellow, white, (t - 0.66) / 0.34)
                }
            } else {
                // Burned -> Dry -> Healthy based on remaining fuel.
                let t = fuel.clamp(0.0, 1.0);

                let ash   = [0.20, 0.19, 0.18];
                let dirt  = [0.34, 0.28, 0.20];
                let grass = [0.28, 0.50, 0.18];
                let forest= [0.10, 0.42, 0.10];

                if t < 0.25 {
                    lerp(ash, dirt, t / 0.25)
                } else if t < 0.6 {
                    lerp(dirt, grass, (t - 0.25) / 0.35)
                } else {
                    lerp(grass, forest, (t - 0.6) / 0.4)
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct CellGrid {
    pub width: usize,
    pub height: usize,
    cells: Vec<Cell>,
}

impl CellGrid {
    fn index_to_coord(&self, idx: usize) -> (usize, usize) {
        let x = idx % self.width;
        let y = (idx - x) / self.width;

        (x, y)
    }

    fn coord_to_index(&self, x: usize, y: usize) -> usize {
        x + y * self.width
    }

    fn neighbors(&self, x: usize, y: usize) -> Vec<Cell> {
        let mut dirs: Vec<(isize, isize)> = vec![];

        let left_edge = x <= 0;
        let right_edge = x >= self.width - 1;
        let top_edge = y <= 0;
        let bottom_edge = y >= self.height - 1;

        if !left_edge {
            dirs.push((-1, 0));
        }
        if !right_edge {
            dirs.push((1, 0));
        }
        if !top_edge {
            dirs.push((0, -1));
        }
        if !bottom_edge {
            dirs.push((0, 1));
        }

        if !left_edge && !top_edge {
            dirs.push((-1, -1));
        }
        if !top_edge && !right_edge {
            dirs.push((1, -1));
        }
        if !right_edge && !bottom_edge {
            dirs.push((1, 1));
        }
        if !bottom_edge && !left_edge {
            dirs.push((-1, 1));
        }

        let mut ns = vec![];

        for d in dirs {
            ns.push(self.cells[self.coord_to_index((x as isize + d.0) as usize, (y as isize + d.1) as usize)].clone());
        }

        ns
    }

    fn wind_neighbors(&self, x: usize, y: usize) -> Vec<(Cell, f32)> {
        let mut cells = Vec::new();

        let (wx, wy) = match self.cells[self.coord_to_index(x, y)] {
            Cell::Land { wind, .. } => wind,
            Cell::Water => return cells,
        };

        let len = (wx * wx + wy * wy).sqrt();

        if len < 0.001 {
            return cells;
        }

        let dx = wx / len;
        let dy = wy / len;

        let steps = len.ceil() as usize;

        for i in 1..=steps {
            let nx = (x as f32 + dx * i as f32).round() as isize;
            let ny = (y as f32 + dy * i as f32).round() as isize;

            if nx >= 0
                && nx < self.width as isize
                && ny >= 0
                && ny < self.height as isize
            {
                cells.push(
                    (self.cells[self.coord_to_index(nx as usize, ny as usize)].clone(), 1.0 / (i as f32))
                );
            }
        }

        cells
    }

    pub fn update(&mut self, dt: f32) -> Self{

        let mut rng = rand::rng();
        let mut next = self.clone();

        for i in 0..self.cells.len() {

            match self.cells[i] {

                Cell::Land { fire: f, o2: oxy, wind: (wx, wy), fuel } => {

                    if !f {
                        //not on fire
                        let (x, y) = self.index_to_coord(i);

                        let mut fire_pressure = 0.0;

                        // Normal neighbors
                        for n in self.neighbors(x, y) {
                            if let Cell::Land { fire: true, .. } = n {
                                fire_pressure += NEIGHBOR_SPREAD_FACTOR;
                            }
                        }

                        // Wind neighbors
                        for (n, weight) in self.wind_neighbors(x, y) {
                            if let Cell::Land { fire: true, .. } = n {
                                fire_pressure += WIND_SPREAD_FACTOR * weight;
                            }
                        }

                        fire_pressure *= SPREAD_FACTOR;

                        let spread_chance = (1.0 - (0.5_f32).powf(dt * fire_pressure)) * fuel;
                        let fire_spread = rng.random_range(0.0..1.0);

                        //raise oxygen + maybe catch on fire
                        let wind_str = wx * wx + wy * wy;
                        let next_oxy = (oxy + dt * OXYGEN_REGROW_FACTOR * wind_str).min(1.0);
                        let mut next_fire = false;
                        if fire_spread < spread_chance {
                            next_fire = true;
                        }

                        next.cells[i] = Cell::Land {fire: next_fire, o2: next_oxy, wind: (wx, wy), fuel};

                    } else {

                        //on fire
                        let ext_chance = 1.0 - (oxy * fuel).powf(dt * EXTINGUISH_FACTOR);

                        let ext = rng.random_range(0.0..1.0);

                        //decrement oxygen + maybe extinguish
                        let wind_str = wx * wx + wy * wy;
                        let next_oxy = (oxy - dt * OXYGEN_BURN_FACTOR / wind_str).max(0.0);
                        let next_fuel = (fuel - dt * FUEL_BURN_FACTOR).max(0.0);
                        let mut next_fire = true;
                        if ext < ext_chance {
                            next_fire = false;
                        }

                        next.cells[i] = Cell::Land {fire: next_fire, o2: next_oxy, wind: (wx, wy), fuel: next_fuel};
                    }

                },
                _ => {}
            }
        }

        next
    }
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

    let mut seen_fire = false;

    for i in 0..(width * height) {

        let height = fbm((i as f32 % width as f32, (i as i32 / width as i32) as f32), 0.1);

        if height < 0.2 {
            cells.push(Cell::Water);
        } else {
            let fire = !seen_fire;

            if fire {
                seen_fire = true;
            }

            let wind = (
                fbm((i as f32 % width as f32 + 500.0, (i as i32 / width as i32) as f32 + 500.0), 0.1) * 10.0 - 5.0, 
                fbm((i as f32 % width as f32 + 1000.0, (i as i32 / width as i32) as f32  + 1000.0), 0.1) * 10.0 - 5.0);

            cells.push(Cell::Land {fire, o2: 1.0, wind, fuel: 1.0});
            
        }
    }

    CellGrid {
        width,
        height,
        cells,
    }
}