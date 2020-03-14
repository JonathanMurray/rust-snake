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
const MOVEMENT_COOLDOWN: f64 = 0.05;
const GRID_SIZE: [i32; 2] = [32, 32];
const CELL_WIDTH: f64 = 16.0;
const COLOR_BG: [f32; 4] = [0.1, 0.1, 0.1, 1.0];
const COLOR_SNAKE: [f32; 4] = [1.0, 1.0, 0.0, 1.0];
const COLOR_DEAD_SNAKE: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
const COLOR_FOOD: [f32; 4] = [0.3, 1.0, 0.3, 0.5];
const COLOR_GRID: [f32; 4] = [0.3, 0.0, 0.7, 1.0];
const PIXEL_OFFSET: [f64; 2] = [
    (WINDOW_SIZE[0] as f64 - GRID_SIZE[0] as f64 * CELL_WIDTH) / 2.0,
    (WINDOW_SIZE[1] as f64 - GRID_SIZE[1] as f64 * CELL_WIDTH) / 2.0,
];

pub struct Game {
    gl: GlGraphics,
    playing: bool,
    snake_positions: Vec<[i32; 2]>,
    next_direction: [i32; 2],
    direction: [i32; 2],
    move_timer: f64,
    food_position: [i32; 2],
}

impl Game {
    fn new(gl: GlGraphics) -> Self {
        Game {
            gl,
            playing: true,
            snake_positions: vec![[0, 0]],
            next_direction: [1, 0],
            direction: [1, 0],
            move_timer: 0.0,
            food_position: [5, 0],
        }
    }

    fn render(&mut self, args: &RenderArgs) {
        let snake_positions = &self.snake_positions;
        let playing = self.playing;
        let food_position = &self.food_position;

        self.gl.draw(args.viewport(), |c, gl| {
            graphics::clear(COLOR_BG, gl);
            let transform = c.transform.trans(PIXEL_OFFSET[0], PIXEL_OFFSET[1]);
            Game::render_grid(transform, gl);
            Game::render_snake(snake_positions, playing, gl, transform);
            Game::render_food(food_position, gl, transform);
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

    fn update(&mut self, args: &UpdateArgs) {
        if self.playing {
            self.move_timer -= args.dt;
            if self.move_timer < 0.0 {
                self.move_timer += MOVEMENT_COOLDOWN;
                let mut new_head = self.snake_positions.last().unwrap().clone();
                self.direction = self.next_direction;
                new_head[0] += self.direction[0];
                new_head[1] += self.direction[1];
                self.snake_positions.push(new_head);
                if self.has_collided() {
                    self.playing = false;
                    println!("GAME OVER")
                }

                if *self.snake_positions.last().unwrap() == self.food_position {
                    let mut rng = rand::thread_rng();
                    let x = rng.gen_range(0, GRID_SIZE[0]);
                    let y = rng.gen_range(0, GRID_SIZE[1]);
                    self.food_position = [x, y];
                } else {
                    self.snake_positions.remove(0);
                }
            }
        }
    }

    fn has_collided(&self) -> bool {
        let head = *self.snake_positions.last().unwrap();
        let outside_grid =
            head[0] < 0 || head[0] >= GRID_SIZE[0] || head[1] < 0 || head[1] >= GRID_SIZE[1];
        let self_collision =
            self.snake_positions[0..self.snake_positions.len() - 1].contains(&head);
        outside_grid || self_collision
    }

    fn handle_key_press(&mut self, key: Key) {
        match key {
            Key::Up => {
                if self.direction != [0, 1] {
                    self.next_direction = [0, -1];
                }
            }
            Key::Down => {
                if self.direction != [0, -1] {
                    self.next_direction = [0, 1];
                }
            }
            Key::Left => {
                if self.direction != [1, 0] {
                    self.next_direction = [-1, 0];
                }
            }
            Key::Right => {
                if self.direction != [-1, 0] {
                    self.next_direction = [1, 0];
                }
            }
            _ => {}
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
    let mut app = Game::new(GlGraphics::new(opengl));

    let mut events = Events::new(EventSettings::new());
    while let Some(e) = events.next(&mut window) {
        if let Some(args) = e.render_args() {
            app.render(&args);
        }

        if let Some(args) = e.update_args() {
            app.update(&args);
        }

        if let Some(args) = e.button_args() {
            if args.state == ButtonState::Press {
                if let Keyboard(key) = args.button {
                    app.handle_key_press(key);
                }
            }
        }
    }
}
