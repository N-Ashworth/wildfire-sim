use crate::noise::{fbm};
use rand::Rng;
use rand::rngs::SmallRng;
use rand::SeedableRng;

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

//--Random numbers / seeding (0 = random each time!)--
const TERRAIN_SEED: u64 = 0;
const FIRE_SEED: u64 = 0;

//--Generation--
const MAX_ELEVATION: f32 = 50.0;
const WATER_LEVEL: f32 = 13.0;

const TERRAIN_SCALE: f32 = 0.05;

//--Fire spread and Extinguish--
const SPREAD_FACTOR: f32 = 1.0; //Constant multiplier on the spread chance

const NEIGHBOR_SPREAD_FACTOR: f32 = 0.2; //Multiplier on the chance of spreading from a neighbor cell
const WIND_SPREAD_FACTOR: f32 = 0.9; //Multiplier on the chance of spreading from a downwind cell

const NEIGHBOR_SPREAD_UPHILL_FACTOR: f32 = 0.00;

const EXTINGUISH_FACTOR: f32 = 1.4; //Constant multiplier on the extinguish chance

const EMBER_CHANCE_FACTOR: f32 = 0.03;

//--Oxygen, Fuel, and Moisture--
const OXYGEN_BURN_FACTOR: f32 = 3.0; //Rate cells lose oxygen while on fire
const OXYGEN_REGROW_FACTOR: f32 = 0.003; //Rate cells regain oxygen not on fire

const FUEL_BURN_FACTOR: f32 = 0.2; //Rate cells burn fuel while on fire

const GLOBAL_WIND_X: f32 = 0.0;
const GLOBAL_WIND_Y: f32 = 5.0;
const THERMAL_WIND_FACTOR: f32 = 1.0; //Multiplier on the effect of fire on the direction and strength of wind
    //(wind goes towards the fire)

const MOISTURE_EVAPORATION_SPEED: f32 = 1.0; //Multiplier on the rate moisture is evaporated next to a fire cell
const BURNING_MOISTURE_EVAPORATION_SPEED: f32 = 1.0; //Multiplier on the rate moisture is burned when the cell is on fire
const MOISTURE_IGNITION_THRESHOLD: f32 = 0.5; //Cells can only catch fire when their moisture is less than this value

fn lerp(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn cell_to_color(grid: &CellGrid, i: usize) -> [f32;3] {
    if grid.cell_land[i] {
        let o2 = grid.cell_o2[i];
        let fuel = grid.cell_fuel[i];
        let moisture = grid.cell_moisture[i];
        let elevation = grid.cell_elevation[i];

        if grid.cell_fire[i] {

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

            let col;

            if heat < 0.25 {
                col = lerp(ember, red, heat/0.25);
            }
            else if heat < 0.55 {
                col = lerp(red, orange, (heat-0.25)/0.3);
            }
            else if heat < 0.8 {
                col = lerp(orange, yellow,(heat-0.55)/0.25);
            }
            else {
                col = lerp(yellow, white,(heat-0.8)/0.2);
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
    } else {
        [0.08,0.25,0.65]
    }
}

#[derive(Clone)]
pub struct CellGrid {
    pub width: usize,
    pub height: usize,
    cell_land: Vec<bool>,
    cell_fire: Vec<bool>,
    cell_o2: Vec<f32>,
    cell_wind: Vec<(f32, f32)>,
    cell_fuel: Vec<f32>,
    cell_elevation: Vec<f32>,
    cell_moisture: Vec<f32>,
    cell_slope_x_neg: Vec<f32>,
    cell_slope_x_pos: Vec<f32>,
    cell_slope_y_neg: Vec<f32>,
    cell_slope_y_pos: Vec<f32>,
    rng: SmallRng,
    embers: Vec<(usize, usize)>,
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

    fn wind_neighbors(&self, x: usize, y: usize) -> Vec<((usize, usize), f32)> {
        let mut cells = Vec::new();

        let (mut wx, mut wy) = self.cell_wind[self.coord_to_index(x, y)];

        wx += GLOBAL_WIND_X;
        wy += GLOBAL_WIND_Y;

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

    fn calculate_slope_effect(&mut self, x1: usize, y1: usize, x2: usize, y2: usize) -> f32 {
        //check if its the same cell
        if x1 == x2 && y1 == y2 { return 0.0; };

        //if the cells are adjacent, use the precomputed value!
        let adjacent = (x1 as isize - x2 as isize).abs() + (y1 as isize - y2 as isize).abs() == 1;

        if adjacent {
            if (x1 as isize - x2 as isize) == 1 {

            }
            if (x1 as isize - x2 as isize) == -1 {
                
            }
            if (y1 as isize - y2 as isize) == 1 {
                
            }
            if (y1 as isize - y2 as isize) == -1 {
                
            }

        }

        //calculate slope, find factor
        let cell1 = self.coord_to_index(x1, y1);
        let cell2 = self.coord_to_index(x2, y2);

        let run_squared = (x1 as f32 - x2 as f32).powi(2) + (y1 as f32 - y2 as f32).powi(2);

        let elev1 = self.cell_elevation[cell1];

        let elev2 = self.cell_elevation[cell2];

        let rise = elev1 - elev2;

        if rise <= 0.0 {
            return 0.0;
        }

        //use rothemels slope factor: 5.275 * tan(theta)^2 (the slope is tan(theta))
        5.275 * (rise * rise) / run_squared
    }

    pub fn update(&mut self, dt: f32) -> Self{

        let mut rng = self.rng.clone();
        let mut next = self.clone();

        next.embers = vec![];

        for i in 0..(self.width * self.height) {

            if self.cell_land[i] {
                let (x, y) = self.index_to_coord(i);
                let wnbrs = self.wind_neighbors(x, y);

                let (wx, wy) = self.cell_wind[i];
                let f = self.cell_fire[i];
                let fuel = self.cell_fuel[i];
                let moisture = self.cell_moisture[i];
                let oxy = self.cell_o2[i];

                if !f {
                    //update wind
                    let mut burning_count = 0;
                    let mut thermal_x = 0.0;
                    let mut thermal_y = 0.0;

                    for dy in -3..=3 {
                        for dx in -3..=3 {
                            if dx == 0 && dy == 0 {
                                continue;
                            }

                            let nx = x as isize + dx;
                            let ny = y as isize + dy;

                            if nx < 0 || ny < 0
                                || nx >= self.width as isize
                                || ny >= self.height as isize
                            {
                                continue;
                            }

                            let ni = nx as usize + ny as usize * self.width;

                            if self.cell_fire[ni] {
                                burning_count += 1;

                                let dist_sq = (dx * dx + dy * dy) as f32;

                                thermal_x += dx as f32 / dist_sq;
                                thermal_y += dy as f32 / dist_sq;
                            }
                        }
                    }
                    let twx = wx + GLOBAL_WIND_X;
                    let twy = wy + GLOBAL_WIND_Y;

                    //not on fire
                    let mut fire_pressure = 0.0;

                    let mut wind_x = wx + thermal_x * THERMAL_WIND_FACTOR * dt;
                    let mut wind_y = wy + thermal_y * THERMAL_WIND_FACTOR * dt;

                    let mut sum_wind = (wind_x, wind_y);
                    let mut n_ct = 1;

                    for dy in -1..=1 {
                        for dx in -1..=1 {
                            if dx == 0 && dy == 0 {
                                continue;
                            }

                            let nx = x as isize + dx;
                            let ny = y as isize + dy;

                            if nx < 0
                                || ny < 0
                                || nx >= self.width as isize
                                || ny >= self.height as isize
                            {
                                continue;
                            }

                            let ni = nx as usize + ny as usize * self.width;

                            let wind = self.cell_wind[ni];
                            sum_wind.0 += wind.0;
                            sum_wind.1 += wind.1;
                            n_ct += 1;

                            if self.cell_fire[ni] {
                                fire_pressure += NEIGHBOR_SPREAD_FACTOR;
                                fire_pressure += self.calculate_slope_effect(
                                    x,
                                    y,
                                    nx as usize,
                                    ny as usize,
                                ) * NEIGHBOR_SPREAD_UPHILL_FACTOR;
                            }
                        }
                    }

                    //Make wind in adjacent cells similar

                    let avg_wind = (sum_wind.0 / (n_ct as f32), sum_wind.1 / (n_ct as f32));

                    //lerp the wind to the average wind slowly - 0.95 current wind per second
                    let alpha = 1.0 - (-1.0 * dt).exp();

                    wind_x += (avg_wind.0 - wind_x) * alpha;
                    wind_y += (avg_wind.1 - wind_y) * alpha;

                    // Wind neighbors
                    for (n, weight) in wnbrs {
                        if self.cell_fire[self.coord_to_index(n.0, n.1)] {
                            fire_pressure += WIND_SPREAD_FACTOR * weight;
                        }
                    }

                    fire_pressure *= SPREAD_FACTOR;

                    let spread_chance = (1.0 - (0.5_f32).powf(dt * fire_pressure)) * fuel;
                    let fire_spread = rng.random_range(0.0..1.0);

                    let heat = fire_pressure + 3.0 * burning_count as f32 / 48.0; 

                    let next_moisture = (moisture - MOISTURE_EVAPORATION_SPEED * heat * dt).max(0.0);

                    //raise oxygen + maybe catch on fire
                    let wind_str = twx * twx + twy * twy;
                    let next_oxy = (oxy + dt * OXYGEN_REGROW_FACTOR * wind_str).min(1.0);
                    let mut next_fire = false;
                    if fire_spread < spread_chance && moisture < MOISTURE_IGNITION_THRESHOLD {
                        next_fire = true;
                    }

                    //Update embers
                    if self.embers.contains(&(x, y)) {
                        next_fire = true;
                    }

                    //next.cells[i] = Cell::Land {fire: next_fire, o2: next_oxy, wind: (wind_x, wind_y), fuel, elevation, moisture: next_moisture};
                    next.cell_fire[i] = next_fire;
                    next.cell_o2[i] = next_oxy;
                    next.cell_wind[i] = (wind_x, wind_y);
                    next.cell_moisture[i] = next_moisture;

                } else {

                    //on fire
                    let (x, y) = self.index_to_coord(i);

                    let ext_chance = 1.0 - (oxy * fuel).powf(dt * EXTINGUISH_FACTOR);

                    let ext = rng.random_range(0.0..1.0);

                    let twx = wx + GLOBAL_WIND_X;
                    let twy = wy + GLOBAL_WIND_Y;

                    //decrement oxygen + maybe extinguish
                    let wind_str = twx * twx + twy * twy;
                    let next_oxy = (oxy - dt * OXYGEN_BURN_FACTOR / wind_str.max(1.0)).max(0.0);
                    let next_fuel = (fuel - dt * FUEL_BURN_FACTOR).max(0.0);
                    let next_moisture = (moisture - BURNING_MOISTURE_EVAPORATION_SPEED * dt).max(0.0);
                    let mut next_fire = true;
                    if ext < ext_chance {
                        next_fire = false;
                    }

                    let mut burning_count = 0;

                    for dy in -3..=3 {
                        for dx in -3..=3 {
                            if dx == 0 && dy == 0 {
                                continue;
                            }

                            let nx = x as isize + dx;
                            let ny = y as isize + dy;

                            if nx < 0 || ny < 0
                                || nx >= self.width as isize
                                || ny >= self.height as isize
                            {
                                continue;
                            }

                            let ni = nx as usize + ny as usize * self.width;

                            if self.cell_fire[ni] {
                                burning_count += 1;
                            }
                        }
                    }

                    //maybe launch an ember into the air
                    let heat = burning_count as f32 + (self.cell_o2[i] * self.cell_fuel[i] - self.cell_moisture[i]) * 10.0 / 34.0;

                    let ember_launch = rng.random_range(0.0..1.0);
                    let ember_chance = 1.0 - (1.0 - heat.powi(3)).powf(dt * EMBER_CHANCE_FACTOR);
                    if ember_launch < ember_chance {
                        let ember_power = rng.random_range(0.3..2.5);
                        next.embers.push((((x as i32) + (twx * ember_power).round() as i32) as usize, ((y as i32 + (twy * ember_power).round() as i32) as i32) as usize));
                    }

                    //next.cells[i] = Cell::Land {fire: next_fire, o2: next_oxy, wind: (wx, wy), fuel: next_fuel, elevation, moisture: next_moisture};
                    next.cell_fire[i] = next_fire;
                    next.cell_o2[i] = next_oxy;
                    next.cell_wind[i] = (wx, wy);
                    next.cell_fuel[i] = next_fuel;
                    next.cell_moisture[i] = next_moisture;
                }
            }
        }
        

        next.rng = rng;
        next
    }

    pub fn start_fire(&mut self, x: usize, y: usize) {
        if x > self.width - 1 || y > self.height - 1 {
            return;
        }
        let idx = self.coord_to_index(x, y);

        self.cell_fire[idx] = true;
    }

    pub fn douse_fire(&mut self, x: usize, y: usize, dt: f32) {
        if x > self.width - 1 || y > self.height - 1 {
            return;
        }
        let idx = self.coord_to_index(x, y);

        self.cell_fire[idx] = false;
        self.cell_moisture[idx] = 1.0;
    }
}

pub fn cell_grid_to_colors(grid: &CellGrid) -> Vec<f32> {
    let mut colors: Vec<f32> = vec![];
    for i in 0..(grid.width * grid.height) {
        colors.extend(Vec::from(cell_to_color(grid, i)));
    }
    colors
}

pub fn gen_grid(width: usize, height: usize) -> CellGrid {
    let mut cell_land: Vec<bool> = vec![];
    let mut cell_elevation: Vec<f32> = vec![];

    let seed = if TERRAIN_SEED > 0 {TERRAIN_SEED} else {rand::random::<u64>()};

    for i in 0..(width * height) {

        let height = fbm((i as f32 % width as f32, (i as i32 / width as i32) as f32), TERRAIN_SCALE, seed);

        if height < WATER_LEVEL / MAX_ELEVATION {
            //cells.push(Cell::Water);
            cell_land.push(false);
            cell_elevation.push(0.0);
        } else {
            //cells.push(Cell::Land {fire: false, o2: 1.0, wind: (0.0, 0.0), fuel: 1.0, elevation: height * MAX_ELEVATION - WATER_LEVEL, moisture: 1.0});
            cell_land.push(true);
            cell_elevation.push(height * MAX_ELEVATION - WATER_LEVEL);
        }
    }

    let mut grid = CellGrid {
        width,
        height,
        cell_land,
        cell_elevation,
        cell_fire: vec![false; width * height],
        cell_fuel: vec![1.0; width * height],
        cell_moisture: vec![1.0; width * height],
        cell_o2: vec![1.0; width * height],
        cell_wind: vec![(0.0, 0.0); width * height],
        cell_slope_x_neg: vec![],
        cell_slope_x_pos: vec![],
        cell_slope_y_neg: vec![],
        cell_slope_y_pos: vec![],
        rng: SmallRng::seed_from_u64(if FIRE_SEED > 0 {FIRE_SEED} else {rand::random::<u64>()}),
        embers: vec![],
    };

    //precalculate cell slopes
    for i in 0..(width * height) {
        let mut slope_neg_x = 0.0;
        let mut slope_pos_x = 0.0;
        let mut slope_neg_y = 0.0;
        let mut slope_pos_y = 0.0;

        let (x, y) = grid.index_to_coord(i);
        let elev = grid.cell_elevation[i];

        if x > 0 {
            slope_neg_x = grid.cell_elevation[grid.coord_to_index(x-1, y)] - elev;
        }
        if x < grid.width - 1 {
            slope_pos_x = grid.cell_elevation[grid.coord_to_index(x+1, y)] - elev;
        }
        if y > 0 {
            slope_neg_y = grid.cell_elevation[grid.coord_to_index(x, y-1)] - elev;
        }
        if y > grid.height - 1 {
            slope_pos_y = grid.cell_elevation[grid.coord_to_index(x, y+1)] - elev;
        }

        grid.cell_slope_x_neg.push(slope_neg_x);
        grid.cell_slope_x_pos.push(slope_pos_x);
        grid.cell_slope_y_neg.push(slope_neg_y);
        grid.cell_slope_y_pos.push(slope_pos_y);
    }

    grid
}