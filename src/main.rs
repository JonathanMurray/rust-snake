extern crate glutin_window;
extern crate graphics;
extern crate opengl_graphics;
extern crate piston;
extern crate rand;

pub mod common;
pub mod entities;

use common::{Color, Direction, Position, CELL_WIDTH};
use entities::{Entity, Snake};
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
const MAX_AMMO: u32 = 5;
const GRID_SIZE: [i32; 2] = [32, 32];
const COLOR_BG: Color = [0.1, 0.1, 0.1, 1.0];
const COLOR_GRID: Color = [0.3, 0.0, 0.7, 1.0];
const PIXEL_OFFSET: [f64; 2] = [
    (WINDOW_SIZE[0] as f64 - GRID_SIZE[0] as f64 * CELL_WIDTH) / 2.0,
    (WINDOW_SIZE[1] as f64 - GRID_SIZE[1] as f64 * CELL_WIDTH) / 2.0,
];

#[derive(Default)]
struct TrapSpawner {
    timer: f64,
    cooldown: f64,
}

impl TrapSpawner {
    fn update(&mut self, elapsed_seconds: f64) -> Option<Position> {
        self.timer -= elapsed_seconds;
        if self.timer < 0.0 {
            self.timer += self.cooldown;
            Some(Game::random_position())
        } else {
            None
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
    trap_spawner: TrapSpawner,
    enemy: Option<Entity>,
    total_elapsed_seconds: f64,
}

impl Game {
    fn new(gl: GlGraphics) -> Self {
        Game {
            gl,
            playing: true,
            snake: Default::default(),
            food: Default::default(),
            bullet: None,
            traps: vec![],
            trap_spawner: TrapSpawner::default(),
            enemy: None,
            total_elapsed_seconds: 0.0,
        }
    }

    fn set_start_state(&mut self) {
        self.playing = true;
        self.snake = Snake::new([0, GRID_SIZE[1] / 2], MAX_AMMO);
        self.food = Entity::new_food(Game::random_position());
        self.bullet = None;
        self.traps = vec![];
        self.enemy = Some(Entity::new_enemy(
            [GRID_SIZE[0] / 2, GRID_SIZE[1] / 2],
            Direction::Down,
        ));
        self.trap_spawner = TrapSpawner {
            timer: 0.0,
            cooldown: 5.0,
        };
        self.total_elapsed_seconds = 0.0;
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
            let timestamp_1 = 30.0;
            let timestamp_2 = 60.0;
            if self.total_elapsed_seconds < timestamp_1
                && self.total_elapsed_seconds + elapsed_seconds >= timestamp_1
            {
                self.trap_spawner.cooldown = 2.0;
            }
            if self.total_elapsed_seconds < timestamp_2
                && self.total_elapsed_seconds + elapsed_seconds >= timestamp_2
            {
                self.trap_spawner.cooldown = 0.5;
            }
            self.total_elapsed_seconds += elapsed_seconds;
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
                    self.snake.gain_ammo(3);
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

            if let Some(trap_position) = self.trap_spawner.update(elapsed_seconds) {
                self.traps.push(Entity::new_trap(trap_position));
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
