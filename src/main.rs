use bevy::prelude::*;
use bevy::render::pass::ClearColor;
use bevy_rapier2d::physics::{
    JointBuilderComponent, RapierConfiguration, RapierPhysicsPlugin, RigidBodyHandleComponent,
};
use bevy_rapier2d::rapier::dynamics::{BallJoint, RigidBodyBuilder, RigidBodySet};
use bevy_rapier2d::rapier::geometry::ColliderBuilder;
use bevy_rapier2d::rapier::na::Vector2;
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
        .add_startup_system(setup_initial_tetromino.system())
        .add_system(tetromino_movement.system())
        .add_system(tetromino_sleep_detection.system())
        .add_plugin(RapierPhysicsPlugin)
        .run();
}

const BLOCK_PX_SIZE: f32 = 30.0;

const MOVEMENT_FORCE: f32 = 10.0;
const TORQUE: f32 = 20.0;

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

fn byte_rgb(r: u8, g: u8, b: u8) -> Color {
    Color::rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
}

fn setup_game(mut game: ResMut<Game>, mut materials: ResMut<Assets<ColorMaterial>>) {
    game.tetromino_colors = vec![
        materials.add(byte_rgb(0, 244, 243).into()),
        materials.add(byte_rgb(238, 243, 0).into()),
        materials.add(byte_rgb(177, 0, 254).into()),
        materials.add(byte_rgb(27, 0, 250).into()),
        materials.add(byte_rgb(252, 157, 0).into()),
        materials.add(byte_rgb(0, 247, 0).into()),
        materials.add(byte_rgb(255, 0, 0).into()),
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

    fn layout(&self) -> TetrominoLayout {
        match self {
            Self::I => TetrominoLayout {
                coords: [(1, 0), (1, 1), (1, 2), (1, 3)],
                joints: vec![(0, 1), (1, 2), (2, 3)],
            },
            Self::O => TetrominoLayout {
                coords: [(0, 0), (1, 0), (1, 1), (0, 1)],
                joints: vec![(0, 1), (1, 2), (2, 3), (1, 0)],
            },
            Self::T => TetrominoLayout {
                coords: [(0, 0), (1, 0), (2, 0), (1, 1)],
                joints: vec![(0, 1), (1, 2), (1, 3)],
            },
            Self::J => TetrominoLayout {
                coords: [(1, 0), (1, 1), (1, 2), (0, 2)],
                joints: vec![(0, 1), (1, 2), (2, 3)],
            },
            Self::L => TetrominoLayout {
                coords: [(1, 0), (1, 1), (1, 2), (2, 2)],
                joints: vec![(0, 1), (1, 2), (2, 3)],
            },
            Self::S => TetrominoLayout {
                coords: [(0, 1), (1, 1), (1, 0), (2, 0)],
                joints: vec![(0, 1), (1, 2), (2, 3)],
            },
            Self::Z => TetrominoLayout {
                coords: [(0, 0), (1, 0), (1, 1), (2, 1)],
                joints: vec![(0, 1), (1, 2), (2, 3)],
            },
        }
    }
}

struct TetrominoLayout {
    coords: [(u8, u8); 4],
    joints: Vec<(u8, u8)>,
}

struct Block;

struct Tetromino {
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
    let kind = TetrominoKind::random();
    let layout = kind.layout();

    let mut blocks: Vec<Entity> = vec![];
    let mut joints: Vec<Entity> = vec![];

    for (x, y) in layout.coords.iter() {
        let lane = (game.n_lanes / 2) - 1 + x;
        let row = game.n_rows - 1 - y;
        let block_entity = spawn_block(commands, game, kind, lane, row);

        blocks.push(block_entity);
    }

    for (i, j) in layout.joints.iter() {
        let prev = blocks[*i as usize];
        let next = blocks[*j as usize];

        let joint = BallJoint::new(Point2::origin(), Point2::new(0.0, 1.0));
        let joint_entity = commands
            .spawn((JointBuilderComponent::new(joint, prev, next),))
            .current_entity()
            .unwrap();
        joints.push(joint_entity);
    }

    let tetromino = Tetromino { blocks, joints };

    commands.spawn((tetromino,));
}

fn tetromino_movement(
    keyboard_input: Res<Input<KeyCode>>,
    tetromino_query: Query<&Tetromino>,
    block_query: Query<&RigidBodyHandleComponent>,
    mut rigid_bodies: ResMut<RigidBodySet>,
) {
    for tetromino in tetromino_query.iter() {
        let mut did_move = false;

        let left_force = if keyboard_input.pressed(KeyCode::Left) {
            did_move = true;
            Some(Vector2::new(-MOVEMENT_FORCE, 0.0))
        } else {
            None
        };

        let right_force = if keyboard_input.pressed(KeyCode::Right) {
            did_move = true;
            Some(Vector2::new(MOVEMENT_FORCE, 0.0))
        } else {
            None
        };

        let counter_clockwise_force = if keyboard_input.pressed(KeyCode::A) {
            did_move = true;
            Some(TORQUE)
        } else {
            None
        };

        let clockwise_force = if keyboard_input.pressed(KeyCode::D) {
            did_move = true;
            Some(-TORQUE)
        } else {
            None
        };

        if did_move {
            for block_entity in &tetromino.blocks {
                if let Ok(rigid_body_component) = block_query.get(*block_entity) {
                    if let Some(rigid_body) = rigid_bodies.get_mut(rigid_body_component.handle()) {
                        if let Some(force) = left_force {
                            rigid_body.apply_force(force, true);
                        }

                        if let Some(force) = right_force {
                            rigid_body.apply_force(force, true);
                        }

                        if let Some(force) = counter_clockwise_force {
                            rigid_body.apply_torque(force, true);
                        }

                        if let Some(force) = clockwise_force {
                            rigid_body.apply_torque(force, true);
                        }
                    }
                }
            }
        }
    }
}

fn tetromino_sleep_detection(
    commands: &mut Commands,
    game: Res<Game>,
    tetromino_query: Query<(Entity, &Tetromino)>,
    block_query: Query<&RigidBodyHandleComponent>,
    rigid_bodies: ResMut<RigidBodySet>,
) {
    for (tetromino_entity, tetromino) in tetromino_query.iter() {
        let all_blocks_sleeping = tetromino.blocks.iter().all(|block_entity| {
            if let Ok(rigid_body_component) = block_query.get(*block_entity) {
                if let Some(rigid_body) = rigid_bodies.get(rigid_body_component.handle()) {
                    rigid_body.is_sleeping()
                } else {
                    false
                }
            } else {
                false
            }
        });

        if all_blocks_sleeping {
            for joint in &tetromino.joints {
                commands.despawn(*joint);
            }
            commands.despawn(tetromino_entity);

            spawn_tetromino(commands, &game);
        }
    }
}
