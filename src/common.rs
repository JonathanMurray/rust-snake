use rand::Rng;

pub type Color = [f32; 4];
pub type Position = [i32; 2];
pub const CELL_WIDTH: f64 = 16.0;

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum Direction {
    Right,
    Left,
    Up,
    Down,
}

impl Default for Direction {
    fn default() -> Self {
        Direction::Right
    }
}

pub fn random_direction() -> Direction {
    let mut rng = rand::thread_rng();
    [
        Direction::Right,
        Direction::Left,
        Direction::Up,
        Direction::Down,
    ][rng.gen_range(0, 4)]
}

impl Direction {
    pub fn as_tuple(&self) -> [i32; 2] {
        match self {
            Direction::Right => [1, 0],
            Direction::Left => [-1, 0],
            Direction::Up => [0, -1],
            Direction::Down => [0, 1],
        }
    }

    pub fn opposite(&self) -> Direction {
        match self {
            Direction::Right => Direction::Left,
            Direction::Left => Direction::Right,
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
        }
    }
}
