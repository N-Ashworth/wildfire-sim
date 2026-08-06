use crate::noise::{fbm};
use rand::Rng;

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
The chance it catches is also related to whether some of the neighbors are downhill -
fire spreads uphill.

3. If a cell is on fire, it spreads its fire in the direction of the wind.
This one is also self-explanatory.

The way the colors are calculated is that

if a cell is water, it's just blue
if a cell is unburned land (o2 = 1.0, fuel = 1.0) then its just green
if a cell is burned land, but it isn't burning, then it fades from green (high fuel) to brown (low fuel)
if a cell is burning, then it fades from white (high oxygen) to red (low oxygen)

note: these are all f32s (floating-point numbers, ones with a decimal)
so if you put 5.0 it will work fine but if you put, say, 5, the compiler
will think it's an int (integer, no decimal) and throw an error.   */

// ----- GLOBAL SIMULATION PARAMETERS -----

//Generation
const MAX_ELEVATION: f32 = 50.0;

//Fire spread and Extinguish
const SPREAD_FACTOR: f32 = 10.0; //Constant multiplier on the spread chance

const NEIGHBOR_SPREAD_FACTOR: f32 = 0.25; //Multiplier on the chance of spreading from a neighbor cell
const WIND_SPREAD_FACTOR: f32 = 0.4; //Multiplier on the chance of spreading from a downwind cell

const NEIGHBOR_SPREAD_UPHILL_FACTOR: f32 = 0.125;
const WIND_SPREAD_UPHILL_FACTOR: f32 = 0.2;

const EXTINGUISH_FACTOR: f32 = 1.0; //Constant multiplier on the extinguish chance

//Oxygen, Fuel, and Moisture
const OXYGEN_BURN_FACTOR: f32 = 3.0; //Rate cells lose oxygen while on fire
const OXYGEN_REGROW_FACTOR: f32 = 0.003; //Rate cells regain oxygen not on fire

const FUEL_BURN_FACTOR: f32 = 0.2; //Rate cells burn fuel while on fire

const THERMAL_WIND_FACTOR: f32 = 1.0; //Multiplier on the effect of fire on the direction and strength of wind
//(wind goes towards the fire)

const MOISTURE_EVAPORATION_SPEED: f32 = 1.0; //Multiplier on the rate moisture is evaporated next to a fire cell
const BURNING_MOISTURE_EVAPORATION_SPEED: f32 = 1.0; //Multiplier on the rate moisture is burned when the cell is on fire
const MOISTURE_IGNITION_THRESHOLD: f32 = 0.5; //Cells can only catch fire when their moisture is less than this value

#[derive(Clone)]
enum Cell {
    Land {
        fire: bool,
        o2: f32,
        wind: (f32, f32),
        fuel: f32,
        elevation: f32,
        moisture: f32,
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

fn cell_to_color(cell: &Cell) -> [f32;3] {
    match *cell {
        Cell::Water => [0.08,0.25,0.65],

        Cell::Land {
            fire,
            o2,
            fuel,
            elevation,
            moisture,
            ..
        } => {

            if fire {

                // Fire intensity
                let heat = (
                    o2 * 0.7 +
                    fuel * 0.3
                ).clamp(0.0,1.0);

                // Wet wood burns darker
                let cooling = moisture.clamp(0.0,1.0);

                let ember = [0.35,0.02,0.0];
                let red = [1.0,0.05,0.0];
                let orange = [1.0,0.45,0.02];
                let yellow = [1.0,0.9,0.2];
                let white = [1.0,1.0,0.8];

                let mut col;

                if heat < 0.25 {
                    col = lerp(ember, red, heat/0.25);
                }
                else if heat < 0.55 {
                    col = lerp(red, orange, (heat-0.25)/0.3);
                }
                else if heat < 0.8 {
                    col = lerp(orange,yellow,(heat-0.55)/0.25);
                }
                else {
                    col = lerp(yellow,white,(heat-0.8)/0.2);
                }


                // moisture reduces brightness
                [
                    col[0] * (1.0 - cooling*0.5),
                    col[1] * (1.0 - cooling*0.7),
                    col[2] * (1.0 - cooling*0.8),
                ]

            } else {

                // dead/burnt terrain
                let fuel_t = fuel.clamp(0.0,1.0);
                let wet_t = moisture.clamp(0.0,1.0);


                let black = [0.02,0.015,0.01];
                let brown = [0.35,0.18,0.05];
                let green = [0.15,0.45,0.08];


                // fuel controls vegetation
                let vegetation =
                    lerp(black,brown,fuel_t);

                // moisture brings brown -> green
                let vegetation =
                    lerp(
                        vegetation,
                        green,
                        wet_t
                    );


                // elevation shading
                let shade =
                    0.6 + 0.4*(elevation/MAX_ELEVATION);

                [
                    vegetation[0]*shade,
                    vegetation[1]*shade,
                    vegetation[2]*shade
                ]
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

    fn neighbors(&self, x: usize, y: usize) -> Vec<(usize, usize)> {
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
            ns.push(((x as isize + d.0) as usize, (y as isize + d.1) as usize));
        }

        ns
    }

    fn wind_neighbors(&self, x: usize, y: usize) -> Vec<((usize, usize), f32)> {
        let mut cells = Vec::new();

        let (wx, wy) = match self.cells[self.coord_to_index(x, y)] {
            Cell::Land { wind, .. } => wind,
            Cell::Water => return cells,
        };

        let len = (wx * wx + wy * wy).sqrt();

        if len < 0.001 {
            return cells;
        }

        let dx = -wx / len;
        let dy = -wy / len;

        let steps = len.ceil() as usize;

        for i in 1..=steps {
            let nx = (x as f32 + dx * i as f32).round() as isize;
            let ny = (y as f32 + dy * i as f32).round() as isize;

            if nx >= 0
                && nx < self.width as isize
                && ny >= 0
                && ny < self.height as isize
            {
                cells.push((
                        (nx as usize, ny as usize), 
                        1.0 / (i as f32)
                ));
            }
        }

        cells
    }

    fn burning_neighbors_in_radius(&self, x: usize, y: usize, r: usize) -> Vec<(usize, usize)> {
        let mut cells: Vec<(usize, usize)> = vec![];

        let borders_x = (
            if r > x {0} else {x - r},
            if r + x > self.width {self.width} else {r + x},
        );
        let borders_y = (
            if r > y {0} else {y - r},
            if r + y > self.height {self.height} else {r + y},
        );
        for nx in borders_x.0..borders_x.1 {
            for ny in borders_y.0..borders_y.1 {
                if nx == x && ny == y {
                    continue;
                }

                match self.cells[self.coord_to_index(nx, ny)] {
                    Cell::Land{ fire: true, .. } => {
                        cells.push((nx, ny));
                    }
                    _ => {}
                }
            }
        }

        cells
    }

    fn calculate_slope_effect(&mut self, x1: usize, y1: usize, x2: usize, y2: usize) -> f32 {
        //calculate slope, find factor
        let cell1 = &self.cells[self.coord_to_index(x1, y1)];
        let cell2 = &self.cells[self.coord_to_index(x2, y2)];

        let run_squared = (x1 as f32 - x2 as f32).powi(2) + (y1 as f32 - y2 as f32).powi(2);

        let elev1 = match cell1 {
            Cell::Land {fire: _, o2: _, wind: _, fuel: _, elevation, moisture: _} => *elevation,
            Cell::Water => 0.0,
        };

        let elev2 = match cell2 {
            Cell::Land {fire: _, o2: _, wind: _, fuel: _, elevation, moisture: _} => *elevation,
            Cell::Water => 0.0,
        };

        let rise = elev1 - elev2;

        if rise <= 0.0 {
            return 0.0;
        }

        //use rothemels slope factor: 5.275 * tan(theta)^2 (the slope is tan(theta))
        5.275 * (rise * rise) / run_squared
    }

    pub fn update(&mut self, dt: f32) -> Self{

        let mut rng = rand::rng();
        let mut next = self.clone();

        for i in 0..self.cells.len() {

            match self.cells[i] {

                Cell::Land { fire: f, o2: oxy, wind: (wx, wy), fuel, elevation, moisture} => {

                    if !f {
                        //not on fire
                        let (x, y) = self.index_to_coord(i);

                        let mut fire_pressure = 0.0;

                        // Normal neighbors
                        for n in self.neighbors(x, y) {

                            if let Cell::Land { fire: true, .. } = self.cells[self.coord_to_index(n.0, n.1)] {
                                fire_pressure += NEIGHBOR_SPREAD_FACTOR;
                                fire_pressure += self.calculate_slope_effect(x, y, n.0, n.1) * NEIGHBOR_SPREAD_UPHILL_FACTOR;
                            }
                        }

                        // Wind neighbors
                        for (n, weight) in self.wind_neighbors(x, y) {
                            if let Cell::Land { fire: true, .. } = self.cells[self.coord_to_index(n.0, n.1)] {
                                fire_pressure += WIND_SPREAD_FACTOR * weight;
                                fire_pressure += self.calculate_slope_effect(x, y, n.0, n.1) * WIND_SPREAD_UPHILL_FACTOR;
                            }
                        }

                        fire_pressure *= SPREAD_FACTOR;

                        let spread_chance = (1.0 - (0.5_f32).powf(dt * fire_pressure)) * fuel;
                        let fire_spread = rng.random_range(0.0..1.0);

                        let next_moisture = (moisture - MOISTURE_EVAPORATION_SPEED * fire_pressure * dt).max(0.0);

                        //raise oxygen + maybe catch on fire
                        let wind_str = wx * wx + wy * wy;
                        let next_oxy = (oxy + dt * OXYGEN_REGROW_FACTOR * wind_str).min(1.0);
                        let mut next_fire = false;
                        if fire_spread < spread_chance && moisture < MOISTURE_IGNITION_THRESHOLD {
                            next_fire = true;
                        }

                        //update wind
                        let mut thermal_wind_x = 0.0;
                        let mut thermal_wind_y = 0.0;

                        for n_coord in self.burning_neighbors_in_radius(x, y, 3) {
                            let dx = n_coord.0 as f32 - x as f32;
                            let dy = n_coord.1 as f32 - y as f32;
                            let dist_sq = dx * dx + dy * dy;
                            
                            // Air rushes TOWARD the fire
                            thermal_wind_x += dx / dist_sq;
                            thermal_wind_y += dy / dist_sq;
                        }

                        let wind_x = wx + thermal_wind_x * THERMAL_WIND_FACTOR * dt;
                        let wind_y = wy + thermal_wind_y * THERMAL_WIND_FACTOR * dt;

                        next.cells[i] = Cell::Land {fire: next_fire, o2: next_oxy, wind: (wind_x, wind_y), fuel, elevation, moisture: next_moisture};

                    } else {

                        //on fire
                        let ext_chance = 1.0 - (oxy * fuel).powf(dt * EXTINGUISH_FACTOR);

                        let ext = rng.random_range(0.0..1.0);

                        //decrement oxygen + maybe extinguish
                        let wind_str = wx * wx + wy * wy;
                        let next_oxy = (oxy - dt * OXYGEN_BURN_FACTOR / wind_str.max(1.0)).max(0.0);
                        let next_fuel = (fuel - dt * FUEL_BURN_FACTOR).max(0.0);
                        let next_moisture = (moisture - BURNING_MOISTURE_EVAPORATION_SPEED * dt).max(0.0);
                        let mut next_fire = true;
                        if ext < ext_chance {
                            next_fire = false;
                        }

                        next.cells[i] = Cell::Land {fire: next_fire, o2: next_oxy, wind: (wx, wy), fuel: next_fuel, elevation, moisture: next_moisture};
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
                -(fbm((i as f32 % width as f32 + 500.0, (i as i32 / width as i32) as f32 + 500.0), 0.1) * 10.0 - 5.0), 
                -(fbm((i as f32 % width as f32 + 1000.0, (i as i32 / width as i32) as f32  + 1000.0), 0.1) * 10.0 - 5.0));

            cells.push(Cell::Land {fire, o2: 1.0, wind, fuel: 1.0, elevation: (height - 0.2) * MAX_ELEVATION, moisture: 1.0});
            
        }
    }

    CellGrid {
        width,
        height,
        cells,
    }
}