use bevy::prelude::*;
use bevy::render::pass::ClearColor;
use bevy_rapier2d::physics::{RapierConfiguration, RapierPhysicsPlugin};
use bevy_rapier2d::rapier::dynamics::RigidBodyBuilder;
use bevy_rapier2d::rapier::geometry::ColliderBuilder;
use rand::Rng;

fn main() {
    App::build()
        .init_resource::<Game>()
        .add_resource(ClearColor(Color::rgb(0.0, 0.0, 0.0)))
        .add_resource(Msaa::default())
        .add_plugins(DefaultPlugins)
        // .add_plugin(bevy_winit::WinitPlugin::default())
        // .add_plugin(bevy_wgpu::WgpuPlugin::default())
        // .add_plugin(RapierRenderPlugin)
        .add_startup_system(setup_env.system())
        .add_startup_system(setup_game.system())
        .add_startup_system(setup_board.system())
        .add_startup_system(setup_test_blocks.system())
        .add_plugin(RapierPhysicsPlugin)
        .run();
}

const BLOCK_PX_SIZE: f32 = 30.0;

struct Game {
    n_lanes: u8,
    n_rows: u8,
    tetromino_colors: Vec<Handle<ColorMaterial>>,
}

impl Game {
    fn floor_y(&self) -> f32 {
        -(self.n_rows as f32) * 0.5
    }

    fn left_wall_x(&self) -> f32 {
        -(self.n_lanes as f32) * 0.5
    }

    fn right_wall_x(&self) -> f32 {
        (self.n_rows as f32) * 0.5
    }
}

impl Default for Game {
    fn default() -> Self {
        Self {
            n_lanes: 4,
            n_rows: 4,
            tetromino_colors: vec![],
        }
    }
}

fn setup_env(commands: &mut Commands, mut rapier_config: ResMut<RapierConfiguration>) {
    // While we want our sprite to look ~40 px square, we want to keep the physics units smaller
    // to prevent float rounding problems. To do this, we set the scale factor in RapierConfiguration
    // and divide our sprite_size by the scale.
    rapier_config.scale = BLOCK_PX_SIZE;

    commands.spawn(Camera2dBundle::default());
}

fn setup_game(mut game: ResMut<Game>, mut materials: ResMut<Assets<ColorMaterial>>) {
    game.tetromino_colors = vec![
        materials.add(Color::rgb(0.0, 0.0, 1.0).into()),
        materials.add(Color::rgb(0.0, 1.0, 0.0).into()),
        materials.add(Color::rgb(0.0, 1.0, 1.0).into()),
        materials.add(Color::rgb(1.0, 0.0, 0.0).into()),
        materials.add(Color::rgb(1.0, 0.0, 1.0).into()),
        materials.add(Color::rgb(1.0, 1.0, 0.0).into()),
        materials.add(Color::rgb(1.0, 1.0, 1.0).into()),
    ];
}

enum TetrominoKind {
    I,
    O,
    T,
    J,
    L,
    S,
    Z,
}

impl TetrominoKind {
    fn random() -> Self {
        match rand::thread_rng().gen_range(0..7) {
            0 => Self::I,
            1 => Self::O,
            2 => Self::T,
            3 => Self::J,
            4 => Self::L,
            5 => Self::S,
            _ => Self::Z,
        }
    }
}

struct Block;

fn setup_board(
    commands: &mut Commands,
    game: Res<Game>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let floor_y = game.floor_y();

    // Add floor
    commands
        .spawn(SpriteBundle {
            material: materials.add(Color::rgb(0.5, 0.5, 0.5).into()),
            sprite: Sprite::new(Vec2::new(
                game.n_lanes as f32 * BLOCK_PX_SIZE,
                BLOCK_PX_SIZE,
            )),
            ..Default::default()
        })
        .with(RigidBodyBuilder::new_static().translation(0.0, floor_y - 0.5))
        .with(ColliderBuilder::cuboid(game.n_lanes as f32 * 0.5, 0.5));
}

fn setup_test_blocks(game: Res<Game>, commands: &mut Commands) {
    for lane in 0..game.n_lanes {
        for row in 0..game.n_rows {
            spawn_block(commands, &game, lane, row + 5);
        }
    }
}

fn spawn_block(commands: &mut Commands, game: &Game, lane: u8, row: u8) {
    let kind = TetrominoKind::random();

    // x, y is the center of the block
    let x = game.left_wall_x() + lane as f32 + 0.5;
    let y = game.floor_y() + row as f32 + 0.5;

    commands
        .spawn(SpriteBundle {
            material: game.tetromino_colors[kind as usize].clone(),
            // transform: Transform::from_translation(Vec3::new(x, y, 1.0)),
            sprite: Sprite::new(Vec2::new(BLOCK_PX_SIZE, BLOCK_PX_SIZE)),
            ..Default::default()
        })
        .with(RigidBodyBuilder::new_dynamic().translation(x, y))
        .with(ColliderBuilder::cuboid(0.5, 0.5).density(100.0))
        .with(Block);
}
