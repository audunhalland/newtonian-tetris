use std::collections::HashSet;

use bevy::prelude::*;
use bevy::render::camera::{OrthographicProjection, ScalingMode};
use bevy_rapier2d::prelude::*;
use rand::Rng;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(Game::new())
        .insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.0)))
        .insert_resource(Msaa::default())
        .add_startup_system(setup_game)
        .add_system(tetromino_movement)
        .add_system(block_death_detection)
        .add_system(tetromino_sleep_detection)
        .add_system(update_health_bar)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .run();
}

// In terms of block size:
const FLOOR_BLOCK_HEIGHT: f32 = 2.0;
const HEALTH_BAR_HEIGHT: f32 = 0.5;

const MOVEMENT_FORCE: f32 = 80.0;
const TORQUE: f32 = 110.0;

#[derive(Default)]
struct Stats {
    generated_blocks: i32,
    cleared_blocks: i32,
    lost_blocks: i32,
    game_over_duration: Option<f32>,
}

impl Stats {
    fn health(&self) -> f32 {
        if self.game_over_duration.is_some() {
            0.0
        } else if self.cleared_blocks == 0 {
            if self.lost_blocks > 0 {
                0.0
            } else {
                1.0
            }
        } else {
            let lost_ratio = self.lost_blocks as f32 / self.cleared_blocks as f32;

            1.0 - lost_ratio
        }
    }
}

#[derive(Resource)]
struct Game {
    n_lanes: usize,
    n_rows: usize,
    stats: Stats,
    current_tetromino_blocks: HashSet<Entity>,
    current_tetromino_joints: Vec<Entity>,
    camera: Option<Entity>,
}

impl Game {
    fn new() -> Self {
        Self {
            n_lanes: 10,
            n_rows: 20,
            stats: Stats::default(),
            current_tetromino_blocks: HashSet::new(),
            current_tetromino_joints: vec![],
            camera: None,
        }
    }

    fn floor_y(&self) -> f32 {
        -(self.n_rows as f32) * 0.5
    }

    fn left_wall_x(&self) -> f32 {
        -(self.n_lanes as f32) * 0.5
    }
}

fn setup_game(mut commands: Commands, mut game: ResMut<Game>) {
    let far = 1000.0;

    let n_rows = game.n_rows as i32;

    game.camera = Some(
        commands
            .spawn(Camera2dBundle {
                projection: OrthographicProjection {
                    far,
                    scaling_mode: ScalingMode::FixedVertical((n_rows + 7) as f32),
                    ..Default::default()
                },
                ..Default::default()
            })
            .id(),
    );

    setup_board(&mut commands, &*game);

    // initial tetromino
    spawn_tetromino(&mut commands, &mut game);
}

#[derive(Clone, Copy, Debug)]
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
                coords: [(1, 1), (1, 0), (1, -1), (1, -2)],
                joints: vec![(0, 1), (1, 2), (2, 3)],
            },
            Self::O => TetrominoLayout {
                coords: [(0, 0), (1, 0), (1, -1), (0, -1)],
                joints: vec![(0, 1), (1, 2), (2, 3), (1, 0)],
            },
            Self::T => TetrominoLayout {
                coords: [(0, 0), (1, 0), (2, 0), (1, -1)],
                joints: vec![(0, 1), (1, 2), (1, 3)],
            },
            Self::J => TetrominoLayout {
                coords: [(1, 0), (1, -1), (1, -2), (0, -2)],
                joints: vec![(0, 1), (1, 2), (2, 3)],
            },
            Self::L => TetrominoLayout {
                coords: [(1, 0), (1, -1), (1, -2), (2, -2)],
                joints: vec![(0, 1), (1, 2), (2, 3)],
            },
            Self::S => TetrominoLayout {
                coords: [(0, -1), (1, -1), (1, 0), (2, 0)],
                joints: vec![(0, 1), (1, 2), (2, 3)],
            },
            Self::Z => TetrominoLayout {
                coords: [(0, 0), (1, 0), (1, -1), (2, -1)],
                joints: vec![(0, 1), (1, 2), (2, 3)],
            },
        }
    }

    fn color(&self) -> Color {
        match self {
            Self::I => Color::rgb_u8(0, 244, 243).into(),
            Self::O => Color::rgb_u8(238, 243, 0).into(),
            Self::T => Color::rgb_u8(177, 0, 254).into(),
            Self::J => Color::rgb_u8(27, 0, 250).into(),
            Self::L => Color::rgb_u8(252, 157, 0).into(),
            Self::S => Color::rgb_u8(0, 247, 0).into(),
            Self::Z => Color::rgb_u8(255, 0, 0).into(),
        }
    }
}

struct TetrominoLayout {
    coords: [(i32, i32); 4],
    joints: Vec<(usize, usize)>,
}

#[derive(Component)]
struct Block;

#[derive(Component)]
struct HealthBar {
    value: f32,
}

fn setup_board(commands: &mut Commands, game: &Game) {
    let floor_y = game.floor_y();

    // Add floor
    commands
        .spawn(SpriteBundle {
            transform: Transform::from_xyz(0.0, floor_y - (FLOOR_BLOCK_HEIGHT - 0.5), 0.0),
            sprite: Sprite {
                color: Color::rgb(0.5, 0.5, 0.5),
                custom_size: Some(Vec2::new(game.n_lanes as f32, FLOOR_BLOCK_HEIGHT)),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(RigidBody::Fixed)
        .insert(Collider::cuboid(
            game.n_lanes as f32 * 0.5,
            FLOOR_BLOCK_HEIGHT * 0.5,
        ));

    // Add health bar
    commands
        .spawn(SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(1.0, 1.0, 1.0),
                custom_size: Some(Vec2::new(game.n_lanes as f32 - 2.0, HEALTH_BAR_HEIGHT)),
                ..Default::default()
            },
            transform: Transform {
                translation: Vec3::new(
                    game.left_wall_x() + 1.0,
                    floor_y - (FLOOR_BLOCK_HEIGHT / 2.0),
                    2.0,
                ),
                rotation: Quat::IDENTITY,
                scale: Vec3::new(0.0, 1.0, 1.0),
            },
            ..Default::default()
        })
        .insert(HealthBar { value: 0.0 });
}

fn spawn_tetromino(commands: &mut Commands, game: &mut Game) {
    let kind = TetrominoKind::random();
    let TetrominoLayout { coords, joints } = kind.layout();

    let block_entities: Vec<Entity> = coords
        .iter()
        .map(|(x, y)| {
            let lane = (game.n_lanes as i32 / 2) - 1 + x;
            let row = game.n_rows as i32 - 1 + y;
            spawn_block(commands, game, kind, lane, row)
        })
        .collect();

    let mut joint_entities: Vec<Entity> = vec![];

    joints.iter().for_each(|(i, j)| {
        let x_dir = coords[*j].0 as f32 - coords[*i].0 as f32;
        let y_dir = coords[*j].1 as f32 - coords[*i].1 as f32;

        let anchor_1 = Vec2::new(x_dir * 0.5, y_dir * 0.5).into();
        let anchor_2 = Vec2::new(x_dir * -0.5, y_dir * -0.5).into();

        let j2 = FixedJointBuilder::new()
            .local_anchor1(anchor_1)
            .local_anchor2(anchor_2);

        commands.entity(block_entities[*j]).with_children(|cmd| {
            let joint_id = cmd.spawn(ImpulseJoint::new(block_entities[*i], j2)).id();

            joint_entities.push(joint_id);
        });
    });

    game.stats.generated_blocks += block_entities.len() as i32;

    game.current_tetromino_blocks = block_entities.into_iter().collect();
    game.current_tetromino_joints = joint_entities;
}

fn spawn_block(
    commands: &mut Commands,
    game: &Game,
    kind: TetrominoKind,
    lane: i32,
    row: i32,
) -> Entity {
    // x, y is the center of the block
    let x = game.left_wall_x() + lane as f32 + 0.5;
    let y = game.floor_y() + row as f32 + 0.5;

    // Game gets more difficult when this is lower:
    let linear_damping = 10.0;

    commands
        .spawn(SpriteBundle {
            transform: Transform::from_xyz(x, y, 0.0),
            sprite: Sprite {
                color: kind.color(),
                // custom_size: Some(Vec2::new(1.0, 1.0)),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(RigidBody::Dynamic)
        .insert(AdditionalMassProperties::Mass(0.2))
        .insert(Damping {
            linear_damping,
            angular_damping: 0.0,
        })
        .insert(Collider::cuboid(0.5, 0.5))
        .insert(Sleeping {
            linear_threshold: 1.0,
            angular_threshold: 1.0,
            sleeping: false,
            ..Default::default()
        })
        // BUG: Rapier does not go to sleep
        // .insert(CustomSleep { duration: 0.0 })
        .insert(ExternalForce::default())
        .insert(Block)
        .id()
}

fn tetromino_movement(
    input: Res<Input<KeyCode>>,
    game: Res<Game>,
    mut external_force: Query<&mut ExternalForce>,
) {
    let movement = input.pressed(KeyCode::Right) as i8 - input.pressed(KeyCode::Left) as i8;
    let torque = input.pressed(KeyCode::A) as i8 - input.pressed(KeyCode::D) as i8;

    for block_entity in &game.current_tetromino_blocks {
        if let Ok(mut forces) = external_force.get_mut(*block_entity) {
            forces.force = Vec2::new(movement as f32 * MOVEMENT_FORCE, 0.0).into();
            forces.torque = torque as f32 * TORQUE;
        }
    }
}

fn tetromino_sleep_detection(
    mut commands: Commands,
    mut game: ResMut<Game>,
    mut block_query: Query<(Entity, &Transform, &mut Sleeping, &RapierRigidBodyHandle)>,
) {
    let all_blocks_sleeping = game.current_tetromino_blocks.iter().all(|block_entity| {
        block_query
            .get(*block_entity)
            .ok()
            .map(|(_, _, sleep, _)| (sleep.sleeping))
            .unwrap_or(false)
    });

    if all_blocks_sleeping {
        for joint in &game.current_tetromino_joints {
            commands.entity(*joint).despawn();
        }

        clear_filled_rows(&mut commands, &mut game, &block_query);

        for (_, _, mut sleeping, _) in &mut block_query {
            sleeping.sleeping = false;
        }

        if game.stats.health() > 0.0 {
            spawn_tetromino(&mut commands, &mut game);
        }
    }
}

fn clear_filled_rows(
    commands: &mut Commands,
    game: &mut Game,
    block_query: &Query<(Entity, &Transform, &mut Sleeping, &RapierRigidBodyHandle)>,
) {
    let mut blocks_per_row: Vec<Vec<Entity>> = (0..game.n_rows).map(|_| vec![]).collect();

    let floor_y = game.floor_y();

    for (block_entity, transform, sleep, _) in block_query.iter() {
        // Only sleeping blocks count.. So disregard blocks "falling off"
        // that are in the row
        if !sleep.sleeping {
            continue;
        }

        let floor_distance = transform.translation.y + 0.5 - floor_y;

        // The center of a block on the floor is 0.5 above the floor, so .floor() the number ;)
        let row = floor_distance.floor() as i32;

        if row >= 0 && row < game.n_rows as i32 {
            blocks_per_row[row as usize].push(block_entity);
        }
    }

    for row_blocks in blocks_per_row {
        if row_blocks.len() == game.n_lanes as usize {
            game.stats.cleared_blocks += game.n_lanes as i32;

            for block_entity in row_blocks {
                commands.entity(block_entity).despawn_recursive();
            }
        }
    }
}

fn block_death_detection(
    mut commands: Commands,
    mut game: ResMut<Game>,
    projection_query: Query<&OrthographicProjection>,
    block_query: Query<(Entity, &Transform, &Block)>,
    time: Res<Time>,
) {
    for projection in projection_query.iter() {
        let outside_limit = projection.area.min.y - 2.0;

        for (block_entity, transform, _) in block_query.iter() {
            if transform.translation.y < outside_limit {
                if game.current_tetromino_blocks.contains(&block_entity) {
                    if game.stats.game_over_duration.is_none() {
                        game.stats.game_over_duration = Some(0.0);
                    }
                }

                game.stats.lost_blocks += 1;
                commands.entity(block_entity).despawn_recursive();
            }
        }
    }

    if let Some(game_over_duration) = game.stats.game_over_duration.as_mut() {
        *game_over_duration += time.delta_seconds();

        // Auto-start new game
        match game.stats.game_over_duration {
            Some(duration) if duration > 3.0 => {
                for (entity, _, _) in block_query.iter() {
                    commands.entity(entity).despawn_recursive();
                }

                game.stats = Default::default();

                spawn_tetromino(&mut commands, &mut game);
            }
            _ => {}
        }
    }
}

fn update_health_bar(
    game: Res<Game>,
    mut health_bar_query: Query<(&mut HealthBar, &mut Transform)>,
) {
    let health = game.stats.health();

    let half_width = (game.n_lanes - 2) as f32 * 0.5;

    for (mut healthbar, mut transform) in health_bar_query.iter_mut() {
        let delta = health - healthbar.value;
        healthbar.value += delta * 0.1;

        transform.translation.x = (game.left_wall_x() + 1.0) + half_width * healthbar.value;
        transform.scale.x = healthbar.value;
    }
}
