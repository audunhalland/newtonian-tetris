use bevy::prelude::*;
use bevy::render::pass::ClearColor;
use bevy_rapier2d::physics::{JointBuilderComponent, RapierConfiguration, RapierPhysicsPlugin};
use bevy_rapier2d::rapier::dynamics::{BallJoint, RigidBodyBuilder};
use bevy_rapier2d::rapier::geometry::ColliderBuilder;
use nalgebra::Point2;
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
        .add_startup_system(setup_initial_tetromino.system())
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
            n_lanes: 8,
            n_rows: 20,
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

#[derive(Clone, Copy)]
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

struct Tetromino {
    kind: TetrominoKind,
    blocks: Vec<Entity>,
    joints: Vec<Entity>,
}

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

fn setup_test_blocks(commands: &mut Commands, game: Res<Game>) {
    for lane in 0..game.n_lanes {
        for row in 0..u8::min(game.n_rows, 2) {
            let kind = TetrominoKind::random();
            spawn_block(commands, &game, kind, lane, row + 2);
        }
    }
}

fn setup_initial_tetromino(commands: &mut Commands, game: Res<Game>) {
    spawn_tetromino(commands, &game);
}

fn spawn_block(
    commands: &mut Commands,
    game: &Game,
    kind: TetrominoKind,
    lane: u8,
    row: u8,
) -> Entity {
    // x, y is the center of the block
    let x = game.left_wall_x() + lane as f32 + 0.5;
    let y = game.floor_y() + row as f32 + 0.5;

    // Game gets more difficult when this is lower:
    let linear_damping = 3.0;

    let rigid_body = RigidBodyBuilder::new_dynamic()
        .translation(x, y)
        .mass(1.0)
        .linear_damping(linear_damping);
    let collider = ColliderBuilder::cuboid(0.5, 0.5).density(1.0);

    commands
        .spawn(SpriteBundle {
            material: game.tetromino_colors[kind as usize].clone(),
            sprite: Sprite::new(Vec2::new(BLOCK_PX_SIZE, BLOCK_PX_SIZE)),
            ..Default::default()
        })
        .with(rigid_body)
        .with(collider)
        .with(Block)
        .current_entity()
        .unwrap()
}

fn spawn_tetromino(commands: &mut Commands, game: &Game) {
    let kind = TetrominoKind::I;
    let lane = game.n_lanes / 2;

    let mut prev_block_entity: Option<Entity> = None;
    let mut blocks: Vec<Entity> = vec![];
    let mut joints: Vec<Entity> = vec![];

    for i in 0..4 {
        let row = game.n_rows - 1 - i;
        let block_entity = spawn_block(commands, game, kind, lane, row);

        blocks.push(block_entity);

        if let Some(prev_block_entity) = prev_block_entity {
            let joint = BallJoint::new(Point2::origin(), Point2::new(0.0, 1.0));
            let joint_entity = commands
                .spawn((JointBuilderComponent::new(
                    joint,
                    prev_block_entity,
                    block_entity.clone(),
                ),))
                .current_entity()
                .unwrap();
            joints.push(joint_entity);
        }

        prev_block_entity = Some(block_entity);
    }

    let tetromino = Tetromino {
        kind,
        blocks,
        joints,
    };

    commands.spawn((tetromino,));
}
