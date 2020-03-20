extern crate glutin_window;
extern crate graphics;
extern crate opengl_graphics;
extern crate piston;
extern crate rand;

use glutin_window::GlutinWindow as Window;
use graphics::types::Matrix2d;
use graphics::Transformed;
use opengl_graphics::{GlGraphics, OpenGL};
use piston::event_loop::{EventSettings, Events};
use piston::input::{RenderArgs, RenderEvent, UpdateArgs, UpdateEvent};
use piston::window::WindowSettings;
use piston::Button::Keyboard;
use piston::ButtonEvent;
use piston::ButtonState;
use piston::Key;
use rand::Rng;
use std::fmt::Debug;

const WINDOW_SIZE: [u32; 2] = [600, 600];
const SNAKE_MOVEMENT_COOLDOWN: f64 = 0.1;
const BULLET_MOVEMENT_COOLDOWN: f64 = 0.07;
const ENEMY_MOVEMENT_COOLDOWN: f64 = 0.3;
const TRAP_SPAWN_COOLDOWN: f64 = 5.0;
const MAX_AMMO: u32 = 5;
const GRID_SIZE: [i32; 2] = [32, 32];
const CELL_WIDTH: f64 = 16.0;
const COLOR_BG: Color = [0.1, 0.1, 0.1, 1.0];
const COLOR_SNAKE: Color = [1.0, 1.0, 0.0, 1.0];
const COLOR_DEAD_SNAKE: Color = [1.0, 0.0, 0.0, 1.0];
const COLOR_FOOD: Color = [0.3, 1.0, 0.3, 0.5];
const COLOR_BULLET: Color = [0.8, 0.1, 0.1, 1.0];
const COLOR_TRAP: Color = [0.8, 0.1, 0.8, 1.0];
const COLOR_ENEMY: Color = [0.4, 0.2, 0.3, 0.8];
const COLOR_GRID: Color = [0.3, 0.0, 0.7, 1.0];
const PIXEL_OFFSET: [f64; 2] = [
    (WINDOW_SIZE[0] as f64 - GRID_SIZE[0] as f64 * CELL_WIDTH) / 2.0,
    (WINDOW_SIZE[1] as f64 - GRID_SIZE[1] as f64 * CELL_WIDTH) / 2.0,
];

type Color = [f32; 4];
type Position = [i32; 2];

#[derive(PartialEq, Copy, Clone, Debug)]
enum Direction {
    Right,
    Left,
    Up,
    Down,
}

fn random_direction() -> Direction {
    let mut rng = rand::thread_rng();
    [
        Direction::Right,
        Direction::Left,
        Direction::Up,
        Direction::Down,
    ][rng.gen_range(0, 4)]
}

impl Direction {
    fn as_tuple(&self) -> [i32; 2] {
        match self {
            Direction::Right => [1, 0],
            Direction::Left => [-1, 0],
            Direction::Up => [0, -1],
            Direction::Down => [0, 1],
        }
    }

    fn opposite(&self) -> Direction {
        match self {
            Direction::Right => Direction::Left,
            Direction::Left => Direction::Right,
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
        }
    }
}

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
            self.direction = random_direction();
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
    position: Position,
    movement: Option<Box<dyn Movement>>,
    color: Color,
}

impl Entity {
    fn new_food(position: Position) -> Self {
        Self {
            position,
            movement: None,
            color: COLOR_FOOD,
        }
    }

    fn new_bullet(position: Position, direction: Direction) -> Self {
        Self {
            position,
            movement: Some(Box::new(StaticMovement::new(
                direction,
                BULLET_MOVEMENT_COOLDOWN,
            ))),
            color: COLOR_BULLET,
        }
    }

    fn new_trap(position: Position) -> Self {
        Self {
            position,
            movement: None,
            color: COLOR_TRAP,
        }
    }

    fn new_enemy(position: Position, direction: Direction) -> Self {
        Self {
            position,
            movement: Some(Box::new(RandomMovement::new(
                direction,
                ENEMY_MOVEMENT_COOLDOWN,
            ))),
            color: COLOR_ENEMY,
        }
    }

    fn update(&mut self, elapsed_seconds: f64) {
        if let Some(movement) = self.movement.as_mut() {
            if let Some([dx, dy]) = movement.apply(elapsed_seconds) {
                self.position = [self.position[0] + dx, self.position[1] + dy];
            }
        }
    }

    fn render(&self, gl: &mut GlGraphics, transform: Matrix2d) {
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

pub struct Snake {
    positions: Vec<Position>,
    next_direction: Direction,
    direction: Direction,
    move_timer: f64,
    ammo: u32,
}

impl Snake {
    fn new(position: Position) -> Self {
        Self {
            positions: vec![position],
            next_direction: Direction::Right,
            direction: Direction::Right,
            move_timer: 0.0,
            ammo: 0,
        }
    }

    fn head(&self) -> Position {
        *self.positions.last().expect("Snake must have head!")
    }

    fn self_collision(&self) -> bool {
        let head = self.head();
        self.positions[0..self.positions.len() - 1].contains(&head)
    }

    fn try_set_direction(&mut self, direction: Direction) {
        if self.direction.opposite() != direction {
            self.next_direction = direction;
        }
    }

    fn update(&mut self, elapsed_seconds: f64) -> bool {
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

    fn render(&self, alive: bool, gl: &mut GlGraphics, transform: Matrix2d) {
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

    fn try_shoot(&mut self) -> Option<(Position, Direction)> {
        if self.ammo > 0 {
            self.ammo -= 1;
            Some((self.position_one_step_forward(), self.direction))
        } else {
            None
        }
    }

    fn gain_ammo(&mut self) {
        if self.ammo < MAX_AMMO {
            self.ammo += 1;
        }
    }
}

pub struct Game {
    gl: GlGraphics,
    playing: bool,
    snake: Snake,
    food: Entity,
    bullet: Option<Entity>,
    traps: Vec<Entity>,
    trap_spawn_timer: f64,
    enemy: Option<Entity>,
}

impl Game {
    fn new(gl: GlGraphics) -> Self {
        Game {
            gl,
            playing: true,
            snake: Snake::new([0, 0]),
            food: Default::default(),
            bullet: None,
            traps: vec![],
            trap_spawn_timer: 0.0,
            enemy: None,
        }
    }

    fn set_start_state(&mut self) {
        self.playing = true;
        self.snake = Snake::new([0, GRID_SIZE[1] / 2]);
        self.food = Entity::new_food(Game::random_position());
        self.bullet = None;
        self.traps = vec![];
        self.enemy = Some(Entity::new_enemy(
            [GRID_SIZE[0] / 2, GRID_SIZE[1] / 2],
            Direction::Down,
        ));
    }

    fn render(&mut self, args: &RenderArgs) {
        let snake = &self.snake;
        let playing = self.playing;
        let food = &self.food;
        let bullet = &self.bullet.as_ref();
        let traps = &self.traps;
        let enemy = &self.enemy.as_ref();
        let ammo = self.snake.ammo;

        self.gl.draw(args.viewport(), |c, gl| {
            graphics::clear(COLOR_BG, gl);
            let transform = c.transform.trans(PIXEL_OFFSET[0], PIXEL_OFFSET[1]);
            Game::render_grid(transform, gl);
            snake.render(playing, gl, transform);
            food.render(gl, transform);
            bullet.map(|bullet| bullet.render(gl, transform));
            for trap in traps {
                trap.render(gl, transform);
            }
            enemy.map(|enemy| enemy.render(gl, transform));
            Game::render_ammo_ui(ammo, gl, transform)
        });
    }

    fn render_ammo_ui(ammo: u32, gl: &mut GlGraphics, transform: Matrix2d) -> () {
        let margin = 4.0;
        let padding = 1.0;
        let width = 16.0;
        for i in 0..MAX_AMMO {
            let square = graphics::rectangle::square(
                -PIXEL_OFFSET[0] + margin + i as f64 * (width + 2.0),
                -PIXEL_OFFSET[1] + margin,
                width,
            );
            graphics::rectangle([0.5, 0.5, 0.5, 1.0], square, transform, gl);
            if ammo > i {
                let square = graphics::rectangle::square(
                    -PIXEL_OFFSET[0] + margin + i as f64 * (width + 2.0) + padding,
                    -PIXEL_OFFSET[1] + margin + padding,
                    width - padding * 2.0,
                );
                graphics::rectangle([0.0, 0.0, 0.0, 1.0], square, transform, gl);
            }
        }
    }

    fn render_grid(transform: Matrix2d, gl: &mut GlGraphics) -> () {
        for y in 0..GRID_SIZE[1] + 1 {
            let y: f64 = y as f64 * CELL_WIDTH;
            graphics::line(
                COLOR_GRID,
                1.0,
                [0.0, y, GRID_SIZE[0] as f64 * CELL_WIDTH, y],
                transform,
                gl,
            );
        }
        for x in 0..GRID_SIZE[0] + 1 {
            let x: f64 = x as f64 * CELL_WIDTH;
            graphics::line(
                COLOR_GRID,
                1.0,
                [x, 0.0, x, GRID_SIZE[1] as f64 * CELL_WIDTH],
                transform,
                gl,
            );
        }
    }

    fn update(&mut self, args: &UpdateArgs) {
        if self.playing {
            let elapsed_seconds = args.dt;
            if let Some(enemy) = self.enemy.as_mut() {
                enemy.update(elapsed_seconds);
            }
            if self.snake.update(elapsed_seconds) {
                let head = self.snake.head();
                if Game::is_outside_grid(&head)
                    || self.snake.self_collision()
                    || self.traps.iter().any(|trap| trap.position == head)
                    || self
                        .enemy
                        .as_ref()
                        .map(|enemy| enemy.position == head)
                        .unwrap_or(false)
                {
                    self.on_game_over()
                }

                if head == self.food.position {
                    self.food.position = Game::random_position();
                    self.snake.gain_ammo();
                } else {
                    self.snake.positions.remove(0);
                }
            }
            if let Some(bullet) = self.bullet.as_mut() {
                bullet.update(elapsed_seconds);
                if bullet.position == self.food.position {
                    self.food.position = Game::random_position();
                }
                self.traps.retain(|trap| trap.position != bullet.position);
            }

            self.trap_spawn_timer -= elapsed_seconds;
            if self.trap_spawn_timer < 0.0 {
                self.trap_spawn_timer += TRAP_SPAWN_COOLDOWN;
                self.traps.push(Entity::new_trap(Game::random_position()));
            }
        }
    }

    fn on_game_over(&mut self) -> () {
        self.traps.clear();
        self.playing = false;
        println!("GAME OVER")
    }

    fn random_position() -> [i32; 2] {
        let mut rng = rand::thread_rng();
        let x = rng.gen_range(0, GRID_SIZE[0]);
        let y = rng.gen_range(0, GRID_SIZE[1]);
        [x, y]
    }

    fn is_outside_grid(position: &Position) -> bool {
        position[0] < 0
            || position[0] >= GRID_SIZE[0]
            || position[1] < 0
            || position[1] >= GRID_SIZE[1]
    }

    fn handle_key_press(&mut self, key: Key) {
        if self.playing {
            match key {
                Key::Up => self.snake.try_set_direction(Direction::Up),
                Key::Down => self.snake.try_set_direction(Direction::Down),
                Key::Left => self.snake.try_set_direction(Direction::Left),
                Key::Right => self.snake.try_set_direction(Direction::Right),
                Key::Space => {
                    if let Some((bullet_position, bullet_direction)) = self.snake.try_shoot() {
                        self.bullet = Some(Entity::new_bullet(bullet_position, bullet_direction));
                    } else {
                        println!("NO AMMO");
                    }
                }
                _ => {}
            }
        } else {
            match key {
                Key::Return => {
                    if !self.playing {
                        println!("RESTARTING");
                        self.set_start_state();
                    }
                }
                _ => {}
            }
        }
    }
}

fn main() {
    // Change this to OpenGL::V2_1 if not working.
    let opengl = OpenGL::V3_2;

    // Create an Glutin window.
    let mut window: Window = WindowSettings::new("RUST SNAKE", WINDOW_SIZE)
        .graphics_api(opengl)
        .exit_on_esc(true)
        .build()
        .expect("Failed to set up window!");

    // Create a new game and run it.
    let mut game = Game::new(GlGraphics::new(opengl));
    game.set_start_state();

    let mut events = Events::new(EventSettings::new());
    while let Some(e) = events.next(&mut window) {
        if let Some(args) = e.render_args() {
            game.render(&args);
        }

        if let Some(args) = e.update_args() {
            game.update(&args);
        }

        if let Some(args) = e.button_args() {
            if args.state == ButtonState::Press {
                if let Keyboard(key) = args.button {
                    game.handle_key_press(key);
                }
            }
        }
    }
}
