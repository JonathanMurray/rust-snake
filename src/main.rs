extern crate glutin_window;
extern crate graphics;
extern crate opengl_graphics;
extern crate piston;
extern crate rand;

use glutin_window::GlutinWindow as Window;
use graphics::Transformed;
use graphics::types::Matrix2d;
use opengl_graphics::{GlGraphics, OpenGL};
use piston::Button::Keyboard;
use piston::ButtonEvent;
use piston::ButtonState;
use piston::event_loop::{Events, EventSettings};
use piston::input::{RenderArgs, RenderEvent, UpdateArgs, UpdateEvent};
use piston::Key;
use piston::window::WindowSettings;
use rand::Rng;

const WINDOW_SIZE: [u32; 2] = [600, 600];
const SNAKE_MOVEMENT_COOLDOWN: f64 = 0.1;
const BULLET_MOVEMENT_COOLDOWN: f64 = 0.07;
const GRID_SIZE: [i32; 2] = [32, 32];
const CELL_WIDTH: f64 = 16.0;
const COLOR_BG: [f32; 4] = [0.1, 0.1, 0.1, 1.0];
const COLOR_SNAKE: [f32; 4] = [1.0, 1.0, 0.0, 1.0];
const COLOR_DEAD_SNAKE: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
const COLOR_FOOD: [f32; 4] = [0.3, 1.0, 0.3, 0.5];
const COLOR_BULLET: [f32; 4] = [0.8, 0.1, 0.1, 1.0];
const COLOR_GRID: [f32; 4] = [0.3, 0.0, 0.7, 1.0];
const PIXEL_OFFSET: [f64; 2] = [
    (WINDOW_SIZE[0] as f64 - GRID_SIZE[0] as f64 * CELL_WIDTH) / 2.0,
    (WINDOW_SIZE[1] as f64 - GRID_SIZE[1] as f64 * CELL_WIDTH) / 2.0,
];

#[derive(PartialEq, Copy, Clone)]
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

pub struct Game {
    gl: GlGraphics,
    playing: bool,
    snake_positions: Vec<[i32; 2]>,
    next_direction: Direction,
    direction: Direction,
    snake_move_timer: f64,
    food_position: [i32; 2],
    bullet_position: Option<[i32; 2]>,
    bullet_move_timer: f64,
    bullet_direction: Direction,
}

impl Game {
    fn new(gl: GlGraphics) -> Self {
        Game {
            gl,
            playing: true,
            snake_positions: vec![],
            next_direction: Direction::Right,
            direction: Direction::Right,
            snake_move_timer: 0.0,
            food_position: [0, 0],
            bullet_position: None,
            bullet_move_timer: 0.0,
            bullet_direction: Direction::Right,
        }
    }

    fn set_start_state(&mut self) {
        self.playing = true;
        self.snake_positions = vec![[0, GRID_SIZE[1] / 2]];
        self.next_direction = Direction::Right;
        self.direction = self.next_direction;
        self.snake_move_timer = 0.0;
        self.spawn_food();
        self.bullet_position = None;
    }

    fn render(&mut self, args: &RenderArgs) {
        let snake_positions = &self.snake_positions;
        let playing = self.playing;
        let food_position = &self.food_position;
        let bullet_position = &self.bullet_position;

        self.gl.draw(args.viewport(), |c, gl| {
            graphics::clear(COLOR_BG, gl);
            let transform = c.transform.trans(PIXEL_OFFSET[0], PIXEL_OFFSET[1]);
            Game::render_grid(transform, gl);
            Game::render_snake(snake_positions, playing, gl, transform);
            Game::render_food(food_position, gl, transform);
            if let Some(pos) = bullet_position {
                Game::render_bullet(&pos, gl, transform);
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
        snake_positions: &Vec<[i32; 2]>,
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

    fn render_food(food_position: &[i32; 2], gl: &mut GlGraphics, transform: Matrix2d) {
        let square = graphics::rectangle::square(
            food_position[0] as f64 * CELL_WIDTH,
            food_position[1] as f64 * CELL_WIDTH,
            CELL_WIDTH,
        );
        graphics::rectangle(COLOR_FOOD, square, transform, gl);
    }

    fn render_bullet(bullet_position: &[i32; 2], gl: &mut GlGraphics, transform: Matrix2d) {
        let square = graphics::rectangle::square(
            bullet_position[0] as f64 * CELL_WIDTH,
            bullet_position[1] as f64 * CELL_WIDTH,
            CELL_WIDTH,
        );
        graphics::rectangle(COLOR_BULLET, square, transform, gl);
    }

    fn update(&mut self, args: &UpdateArgs) {
        if self.playing {
            self.snake_move_timer -= args.dt;
            if self.snake_move_timer < 0.0 {
                self.snake_move_timer += SNAKE_MOVEMENT_COOLDOWN;
                let mut new_head = self.snake_head().clone();
                self.direction = self.next_direction;
                new_head[0] += self.direction.as_tuple()[0];
                new_head[1] += self.direction.as_tuple()[1];
                self.snake_positions.push(new_head);
                if self.has_collided() {
                    self.playing = false;
                    println!("GAME OVER")
                }

                if self.snake_head() == self.food_position {
                    self.spawn_food();
                } else {
                    self.snake_positions.remove(0);
                }
            }
            if let Some(bullet_pos) = self.bullet_position {
                self.bullet_move_timer -= args.dt;
                if self.bullet_move_timer < 0.0 {
                    self.bullet_move_timer += BULLET_MOVEMENT_COOLDOWN;
                    let [dx, dy] = self.bullet_direction.as_tuple();
                    self.bullet_position = Some([bullet_pos[0] + dx, bullet_pos[1] + dy]);
                }

                if bullet_pos == self.food_position {
                    self.spawn_food();
                }
            }
        }
    }

    fn spawn_food(&mut self) {
        let mut rng = rand::thread_rng();
        let x = rng.gen_range(0, GRID_SIZE[0]);
        let y = rng.gen_range(0, GRID_SIZE[1]);
        self.food_position = [x, y];
    }

    fn snake_head(&self) -> [i32; 2] {
        *self.snake_positions.last().unwrap()
    }

    fn has_collided(&self) -> bool {
        let head = self.snake_head();
        let outside_grid =
            head[0] < 0 || head[0] >= GRID_SIZE[0] || head[1] < 0 || head[1] >= GRID_SIZE[1];
        let self_collision =
            self.snake_positions[0..self.snake_positions.len() - 1].contains(&head);
        outside_grid || self_collision
    }

    fn handle_direction_key_press(&mut self, pressed_direction: Direction) {
        if self.direction.opposite() != pressed_direction {
            self.next_direction = pressed_direction;
        }
    }

    fn handle_key_press(&mut self, key: Key) {
        if self.playing {
            match key {
                Key::Up => self.handle_direction_key_press(Direction::Up),
                Key::Down => self.handle_direction_key_press(Direction::Down),
                Key::Left => self.handle_direction_key_press(Direction::Left),
                Key::Right => self.handle_direction_key_press(Direction::Right),
                Key::Space => {
                    let head = self.snake_head();
                    self.bullet_direction = self.direction;
                    let [dx, dy] = self.bullet_direction.as_tuple();
                    self.bullet_position = Some([head[0] + dx, head[1] + dy]);
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
        .unwrap();

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
