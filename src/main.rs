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

const WINDOW_SIZE: [u32; 2] = [600, 600];
const SNAKE_MOVEMENT_COOLDOWN: f64 = 0.1;
const BULLET_MOVEMENT_COOLDOWN: f64 = 0.07;
const TRAP_SPAWN_COOLDOWN: f64 = 5.0;

const GRID_SIZE: [i32; 2] = [32, 32];
const CELL_WIDTH: f64 = 16.0;
const COLOR_BG: Color = [0.1, 0.1, 0.1, 1.0];
const COLOR_SNAKE: Color = [1.0, 1.0, 0.0, 1.0];
const COLOR_DEAD_SNAKE: Color = [1.0, 0.0, 0.0, 1.0];
const COLOR_FOOD: Color = [0.3, 1.0, 0.3, 0.5];
const COLOR_BULLET: Color = [0.8, 0.1, 0.1, 1.0];
const COLOR_TRAP: Color = [0.8, 0.1, 0.8, 1.0];
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

#[derive(Debug)]
pub struct EntityMovement {
    timer: f64,
    direction: Direction,
    cooldown: f64,
}

impl EntityMovement {
    fn new(direction: Direction, cooldown: f64) -> Self {
        Self {
            timer: 0.0,
            direction,
            cooldown,
        }
    }
}

#[derive(Debug)]
pub struct Entity {
    position: Position,
    movement: Option<EntityMovement>,
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
            movement: Some(EntityMovement::new(direction, BULLET_MOVEMENT_COOLDOWN)),
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

    fn update(&mut self, elapsed_seconds: f64) {
        if let Some(movement) = self.movement.as_mut() {
            movement.timer -= elapsed_seconds;
            if movement.timer < 0.0 {
                movement.timer += movement.cooldown;
                let [dx, dy] = movement.direction.as_tuple();
                self.position = [self.position[0] + dx, self.position[1] + dy];
            }
        }
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
}

impl Snake {
    fn new(position: Position) -> Self {
        Self {
            positions: vec![position],
            next_direction: Direction::Right,
            direction: Direction::Right,
            move_timer: 0.0,
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

    fn position_one_step_forward(& self) -> Position {
        let head = self.head();
        let [dx, dy] = self.direction.as_tuple();
        [head[0] + dx, head[1] + dy]
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
        }
    }

    fn set_start_state(&mut self) {
        self.playing = true;
        self.snake = Snake::new([0, GRID_SIZE[1] / 2]);
        self.food = Entity::new_food(Game::random_position());
        self.bullet = None;
        self.traps = vec![];
    }

    fn render(&mut self, args: &RenderArgs) {
        let snake = &self.snake;
        let playing = self.playing;
        let food = &self.food;
        let bullet = &self.bullet.as_ref();
        let traps = &self.traps;

        self.gl.draw(args.viewport(), |c, gl| {
            graphics::clear(COLOR_BG, gl);
            let transform = c.transform.trans(PIXEL_OFFSET[0], PIXEL_OFFSET[1]);
            Game::render_grid(transform, gl);
            Game::render_snake(&snake.positions, playing, gl, transform);
            Game::render_entity(food, gl, transform);
            bullet.map(|bullet| Game::render_entity(&bullet, gl, transform));
            for trap in traps {
                Game::render_entity(trap, gl, transform);
            }
        });
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

    fn render_snake(
        snake_positions: &Vec<Position>,
        playing: bool,
        gl: &mut GlGraphics,
        transform: Matrix2d,
    ) {
        let color = if playing {
            COLOR_SNAKE
        } else {
            COLOR_DEAD_SNAKE
        };
        for pos in snake_positions {
            let square = graphics::rectangle::square(
                pos[0] as f64 * CELL_WIDTH,
                pos[1] as f64 * CELL_WIDTH,
                CELL_WIDTH,
            );
            graphics::rectangle(color, square, transform, gl);
        }
    }

    fn render_entity(entity: &Entity, gl: &mut GlGraphics, transform: Matrix2d) {
        let square = graphics::rectangle::square(
            entity.position[0] as f64 * CELL_WIDTH,
            entity.position[1] as f64 * CELL_WIDTH,
            CELL_WIDTH,
        );
        graphics::rectangle(entity.color, square, transform, gl);
    }

    fn update(&mut self, args: &UpdateArgs) {
        if self.playing {
            if self.snake.update(args.dt) {
                let head = self.snake.head();
                if Game::is_outside_grid(&head)
                    || self.snake.self_collision()
                    || self.traps.iter().any(|trap| trap.position == head)
                {
                    self.on_game_over()
                }

                if self.snake.head() == self.food.position {
                    self.food.position = Game::random_position();
                } else {
                    self.snake.positions.remove(0);
                }
            }
            if let Some(bullet) = self.bullet.as_mut() {
                bullet.update(args.dt);

                if bullet.position == self.food.position {
                    self.food.position = Game::random_position();
                }
                self.traps.retain(|trap| trap.position != bullet.position);
            }
            self.trap_spawn_timer -= args.dt;
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
                    let bullet_position = self.snake.position_one_step_forward();
                    self.bullet = Some(Entity::new_bullet(bullet_position, self.snake.direction));
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
