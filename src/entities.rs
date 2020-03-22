use common;
use common::Direction;
use common::{Color, Position, CELL_WIDTH};
use graphics::types::Matrix2d;
use opengl_graphics::GlGraphics;
use std::fmt::Debug;

const COLOR_SNAKE: Color = [1.0, 1.0, 0.0, 1.0];
const COLOR_DEAD_SNAKE: Color = [1.0, 0.0, 0.0, 1.0];
const COLOR_FOOD: Color = [0.3, 1.0, 0.3, 0.5];
const COLOR_BULLET: Color = [0.8, 0.1, 0.1, 1.0];
const COLOR_TRAP: Color = [0.8, 0.1, 0.8, 1.0];
const COLOR_ENEMY: Color = [0.4, 0.2, 0.3, 0.8];
const SNAKE_MOVEMENT_COOLDOWN: f64 = 0.1;
const BULLET_MOVEMENT_COOLDOWN: f64 = 0.07;
const ENEMY_MOVEMENT_COOLDOWN: f64 = 0.3;

trait Movement: Debug {
    fn apply(&mut self, elapsed_seconds: f64) -> Option<[i32; 2]>;
}

#[derive(Debug)]
pub struct RandomMovement {
    timer: f64,
    direction: Direction,
    cooldown: f64,
}

impl RandomMovement {
    fn new(direction: Direction, cooldown: f64) -> Self {
        Self {
            timer: 0.0,
            direction,
            cooldown,
        }
    }
}

impl Movement for RandomMovement {
    fn apply(&mut self, elapsed_seconds: f64) -> Option<[i32; 2]> {
        self.timer -= elapsed_seconds;
        if self.timer < 0.0 {
            self.timer += self.cooldown;
            self.direction = common::random_direction();
            Some(self.direction.as_tuple())
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct StaticMovement {
    timer: f64,
    direction: Direction,
    cooldown: f64,
}

impl StaticMovement {
    fn new(direction: Direction, cooldown: f64) -> Self {
        Self {
            timer: 0.0,
            direction,
            cooldown,
        }
    }
}

impl Movement for StaticMovement {
    fn apply(&mut self, elapsed_seconds: f64) -> Option<[i32; 2]> {
        self.timer -= elapsed_seconds;
        if self.timer < 0.0 {
            self.timer += self.cooldown;
            Some(self.direction.as_tuple())
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct Entity {
    pub position: Position,
    movement: Option<Box<dyn Movement>>,
    color: Color,
}

impl Entity {
    pub fn new_food(position: Position) -> Self {
        Self {
            position,
            movement: None,
            color: COLOR_FOOD,
        }
    }

    pub fn new_bullet(position: Position, direction: Direction) -> Self {
        Self {
            position,
            movement: Some(Box::new(StaticMovement::new(
                direction,
                BULLET_MOVEMENT_COOLDOWN,
            ))),
            color: COLOR_BULLET,
        }
    }

    pub fn new_trap(position: Position) -> Self {
        Self {
            position,
            movement: None,
            color: COLOR_TRAP,
        }
    }

    pub fn new_enemy(position: Position, direction: Direction) -> Self {
        Self {
            position,
            movement: Some(Box::new(RandomMovement::new(
                direction,
                ENEMY_MOVEMENT_COOLDOWN,
            ))),
            color: COLOR_ENEMY,
        }
    }

    pub fn update(&mut self, elapsed_seconds: f64) {
        if let Some(movement) = self.movement.as_mut() {
            if let Some([dx, dy]) = movement.apply(elapsed_seconds) {
                self.position = [self.position[0] + dx, self.position[1] + dy];
            }
        }
    }

    pub fn render(&self, gl: &mut GlGraphics, transform: Matrix2d) {
        let square = graphics::rectangle::square(
            self.position[0] as f64 * CELL_WIDTH,
            self.position[1] as f64 * CELL_WIDTH,
            CELL_WIDTH,
        );
        graphics::rectangle(self.color, square, transform, gl);
    }
}

impl Default for Entity {
    fn default() -> Self {
        Self {
            position: [0, 0],
            movement: None,
            color: [0.0, 0.0, 0.0, 1.0],
        }
    }
}

#[derive(Default)]
pub struct Snake {
    pub positions: Vec<Position>,
    next_direction: Direction,
    direction: Direction,
    move_timer: f64,
    pub ammo: u32,
    max_ammo: u32,
}

impl Snake {
    pub fn new(position: Position, max_ammo: u32) -> Self {
        Self {
            positions: vec![position],
            next_direction: Direction::Right,
            direction: Direction::Right,
            move_timer: 0.0,
            ammo: 0,
            max_ammo,
        }
    }

    pub fn head(&self) -> Position {
        *self.positions.last().expect("Snake must have head!")
    }

    pub fn self_collision(&self) -> bool {
        let head = self.head();
        self.positions[0..self.positions.len() - 1].contains(&head)
    }

    pub fn try_set_direction(&mut self, direction: Direction) {
        if self.direction.opposite() != direction {
            self.next_direction = direction;
        }
    }

    pub fn update(&mut self, elapsed_seconds: f64) -> bool {
        self.move_timer -= elapsed_seconds;
        if self.move_timer < 0.0 {
            self.move_timer += SNAKE_MOVEMENT_COOLDOWN;
            self.direction = self.next_direction;
            let new_head = self.position_one_step_forward();
            self.positions.push(new_head);
            true
        } else {
            false
        }
    }

    fn position_one_step_forward(&self) -> Position {
        let head = self.head();
        let [dx, dy] = self.direction.as_tuple();
        [head[0] + dx, head[1] + dy]
    }

    pub fn render(&self, alive: bool, gl: &mut GlGraphics, transform: Matrix2d) {
        let color = if alive { COLOR_SNAKE } else { COLOR_DEAD_SNAKE };
        for pos in &self.positions {
            let square = graphics::rectangle::square(
                pos[0] as f64 * CELL_WIDTH,
                pos[1] as f64 * CELL_WIDTH,
                CELL_WIDTH,
            );
            graphics::rectangle(color, square, transform, gl);
        }
    }

    pub fn try_shoot(&mut self) -> Option<(Position, Direction)> {
        if self.ammo > 0 {
            self.ammo -= 1;
            Some((self.position_one_step_forward(), self.direction))
        } else {
            None
        }
    }

    pub fn gain_ammo(&mut self, amount: u32) {
        self.ammo = std::cmp::min(self.ammo + amount, self.max_ammo);
    }
}
