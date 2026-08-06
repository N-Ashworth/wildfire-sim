fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

// Simple deterministic hash returning [0,1)
fn hash(x: i32, y: i32) -> f32 {
    let mut n = x
        .wrapping_mul(374761393)
        .wrapping_add(y.wrapping_mul(668265263));

    n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    ((n ^ (n >> 16)) as u32) as f32 / u32::MAX as f32
}

fn value_noise(pos: (f32, f32), scale: f32) -> f32 {
    let x = pos.0 * scale;
    let y = pos.1 * scale;

    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;

    let x1 = x0 + 1;
    let y1 = y0 + 1;

    let tx = smoothstep(x - x.floor());
    let ty = smoothstep(y - y.floor());

    let bl = hash(x0, y0);
    let br = hash(x1, y0);
    let tl = hash(x0, y1);
    let tr = hash(x1, y1);

    let bottom = lerp(bl, br, tx);
    let top = lerp(tl, tr, tx);

    lerp(bottom, top, ty)
}

pub fn fbm(pos: (f32, f32), scale: f32) -> f32 {
    let mut value = 0.0;
    let mut max = 0.0;
    let mut amplitude = 0.5;
    let mut frequency = 1.0;

    for _ in 0..6 {
        value += amplitude * value_noise(pos, frequency * scale);
        max += amplitude;
        frequency *= 2.0;
        amplitude *= 0.5;
    }

    value / max
}