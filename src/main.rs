use std::{collections::HashSet, convert::TryFrom};

use bevy::prelude::*;
use bevy::render::camera::OrthographicProjection;
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
        .add_startup_system(setup_rapier.system())
        .add_startup_system(setup_game.system())
        .add_startup_system(setup_board.system())
        .add_startup_system(setup_initial_tetromino.system())
        .add_system(tetromino_movement.system())
        .add_system(block_death_detection.system())
        .add_system(tetromino_sleep_detection.system())
        .add_plugin(RapierPhysicsPlugin)
        .run();
}

const BLOCK_PX_SIZE: f32 = 30.0;

const MOVEMENT_FORCE: f32 = 20.0;
const TORQUE: f32 = 20.0;

struct Game {
    n_lanes: u8,
    n_rows: u8,
    tetromino_colors: Vec<Handle<ColorMaterial>>,
    current_tetromino_blocks: HashSet<Entity>,
    current_tetromino_joints: Vec<Entity>,
    camera: Option<Entity>,
}

impl Game {
    fn floor_y(&self) -> f32 {
        -(self.n_rows as f32) * 0.5
    }

    fn left_wall_x(&self) -> f32 {
        -(self.n_lanes as f32) * 0.5
    }
}

impl Default for Game {
    fn default() -> Self {
        Self {
            n_lanes: 8,
            n_rows: 20,
            tetromino_colors: vec![],
            current_tetromino_blocks: HashSet::new(),
            current_tetromino_joints: vec![],
            camera: None,
        }
    }
}

fn setup_rapier(mut rapier_config: ResMut<RapierConfiguration>) {
    // While we want our sprite to look ~40 px square, we want to keep the physics units smaller
    // to prevent float rounding problems. To do this, we set the scale factor in RapierConfiguration
    // and divide our sprite_size by the scale.
    rapier_config.scale = BLOCK_PX_SIZE;
}

fn byte_rgb(r: u8, g: u8, b: u8) -> Color {
    Color::rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
}

fn setup_game(
    commands: &mut Commands,
    mut game: ResMut<Game>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    game.tetromino_colors = vec![
        materials.add(byte_rgb(0, 244, 243).into()),
        materials.add(byte_rgb(238, 243, 0).into()),
        materials.add(byte_rgb(177, 0, 254).into()),
        materials.add(byte_rgb(27, 0, 250).into()),
        materials.add(byte_rgb(252, 157, 0).into()),
        materials.add(byte_rgb(0, 247, 0).into()),
        materials.add(byte_rgb(255, 0, 0).into()),
    ];

    game.camera = commands.spawn(Camera2dBundle::default()).current_entity();
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

fn setup_initial_tetromino(commands: &mut Commands, mut game: ResMut<Game>) {
    spawn_tetromino(commands, &mut game);
}

fn spawn_tetromino(commands: &mut Commands, game: &mut Game) {
    let kind = TetrominoKind::random();
    let layout = kind.layout();

    let mut blocks: Vec<Entity> = vec![];

    for (x, y) in layout.coords.iter() {
        let lane = (game.n_lanes / 2) - 1 + x;
        let row = game.n_rows - 1 - y;
        let block_entity = spawn_block(commands, game, kind, lane, row);

        blocks.push(block_entity);
    }

    let mut joints: Vec<Entity> = vec![];

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

    game.current_tetromino_blocks = blocks.into_iter().collect();
    game.current_tetromino_joints = joints;
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

fn tetromino_movement(
    keyboard_input: Res<Input<KeyCode>>,
    game: Res<Game>,
    block_query: Query<&RigidBodyHandleComponent>,
    mut rigid_bodies: ResMut<RigidBodySet>,
) {
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
        for block_entity in &game.current_tetromino_blocks {
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

fn tetromino_sleep_detection(
    commands: &mut Commands,
    mut game: ResMut<Game>,
    block_query: Query<(Entity, &RigidBodyHandleComponent)>,
    rigid_bodies: ResMut<RigidBodySet>,
) {
    let all_blocks_sleeping = game.current_tetromino_blocks.iter().all(|block_entity| {
        if let Ok((_, rigid_body_component)) = block_query.get(*block_entity) {
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
        for joint in &game.current_tetromino_joints {
            commands.despawn(*joint);
        }

        clear_filled_rows(commands, &mut game, block_query, &rigid_bodies);

        spawn_tetromino(commands, &mut game);

        println!("rigid_body_count: {}", rigid_bodies.len());
    }
}

fn clear_filled_rows(
    commands: &mut Commands,
    game: &mut Game,
    block_query: Query<(Entity, &RigidBodyHandleComponent)>,
    rigid_bodies: &RigidBodySet,
) {
    let mut blocks_per_row: Vec<Vec<Entity>> = (0..game.n_rows).map(|_| vec![]).collect();

    let floor_y = game.floor_y();

    for (block_entity, rigid_body_component) in block_query.iter() {
        if let Some(rigid_body) = rigid_bodies.get(rigid_body_component.handle()) {
            // Only sleeping blocks count.. So disregard blocks "falling off"
            // that are in the row
            if !rigid_body.is_sleeping() {
                continue;
            }

            let floor_distance = rigid_body.position().translation.vector.y - floor_y;

            // The center of a block on the floor is 0.5 above the floor, so .floor() the number ;)
            let row = floor_distance.floor() as i32;

            match u8::try_from(row) {
                Ok(row) => {
                    if row < game.n_rows {
                        blocks_per_row[row as usize].push(block_entity);
                    }
                }
                Err(_) => {}
            }
        }
    }

    for row_blocks in blocks_per_row {
        if row_blocks.len() == game.n_lanes as usize {
            for block_entity in row_blocks {
                commands.despawn(block_entity);
            }
        }
    }
}

fn block_death_detection(
    commands: &mut Commands,
    game: ResMut<Game>,
    projection_query: Query<&OrthographicProjection>,
    block_query: Query<(Entity, &Transform, &Block)>,
) {
    for projection in projection_query.iter() {
        let outside_limit = projection.bottom - BLOCK_PX_SIZE * 2.0;

        for (block_entity, transform, _) in block_query.iter() {
            if transform.translation.y < outside_limit {
                if game.current_tetromino_blocks.contains(&block_entity) {
                    // TODO: Tetromino died. Game over.
                }

                commands.despawn(block_entity);
            }
        }
    }
}
