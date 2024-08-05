use bevy::{
    core::TaskPoolThreadAssignmentPolicy, pbr::ClusterConfig, prelude::*,
    tasks::available_parallelism, window::PresentMode,
};
use bevy_screen_diagnostics::{ScreenDiagnosticsPlugin, ScreenFrameDiagnosticsPlugin};
// use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy_turborand::prelude::*;
use serde::Deserialize;

use std::{f32::consts::PI, time::Duration};

const COUNT: usize = 20000;
const SIZE: f32 = 100.0;
const MOVESPEED: f32 = 5.0;

#[derive(Resource, Deserialize)]
pub struct Configuration {
    count: usize,
    size: f32,
    speed: f32,
    camera: Vec3,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let mut config = Configuration {
        count: COUNT,
        size: SIZE,
        speed: MOVESPEED,
        camera: Vec3 {
            x: 0.0,
            y: 90.0,
            z: 90.0,
        },
    };

    if args.len() >= 2 {
        let config_path = &args[1];
        let contents = std::fs::read_to_string(config_path).unwrap();
        config = serde_json::from_str(&contents).unwrap();
    }

    App::new()
        .insert_resource(config)
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        resolution: (400., 300.).into(),
                        present_mode: PresentMode::Immediate,
                        ..default()
                    }),
                    ..default()
                })
                .set(TaskPoolPlugin {
                    task_pool_options: TaskPoolOptions {
                        compute: TaskPoolThreadAssignmentPolicy {
                            // set the minimum # of compute threads
                            // to the total number of available threads
                            min_threads: available_parallelism(),
                            max_threads: std::usize::MAX, // unlimited max threads
                            percent: 1.0,                 // this value is irrelevant in this case
                        },
                        // keep the defaults for everything else
                        ..default()
                    },
                }),
        )
        .add_plugins(ScreenDiagnosticsPlugin::default())
        .add_plugins(ScreenFrameDiagnosticsPlugin)
        // .add_plugins((LogDiagnosticsPlugin::default(), FrameTimeDiagnosticsPlugin))
        .add_plugins(RngPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Startup, place_items)
        .add_systems(Startup, place_robots)
        .add_systems(Update, robot_target_system)
        .add_systems(Update, robot_move_to_carry_system)
        .add_systems(Update, robot_move_to_drop_system)
        .add_systems(Update, robot_cooldown_system)
        .run();
}

fn setup(
    mut commands: Commands,
    config: Res<Configuration>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // plane base
    commands.spawn(PbrBundle {
        // mesh: meshes.add(Mesh::from(shape::Plane::from_size(config.size))),
        mesh: meshes.add(Mesh::from(
            Plane3d::default().mesh().size(config.size, config.size),
        )),

        material: materials.add(Color::srgb(0.3, 0.5, 0.3)),
        ..default()
    });

    // light
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: false,
            ..default()
        },
        transform: Transform {
            translation: Vec3::new(0.0, 2.0, 0.0),
            rotation: Quat::from_rotation_x(-PI / 4.),
            ..default()
        },
        ..default()
    });

    // camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(config.camera.x, config.camera.y, config.camera.z)
                .looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        ClusterConfig::Single,
    ));
}

#[derive(Component)]
pub struct Robot {}

#[derive(Component)]
pub struct RobotTarget {}

#[derive(Component)]
pub struct AttachedToRobot {}

#[derive(Component)]
pub struct CarryTarget {
    pub item: Entity,
    pub position: Vec3,
}

#[derive(Component)]
pub struct DropTarget {
    pub position: Vec3,
}

#[derive(Component)]
pub struct Cooldown {
    pub time_left: Duration,
}

fn place_items(
    config: Res<Configuration>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut global_rng: ResMut<GlobalRng>,
) {
    let mesh = meshes.add(Cuboid::from_length(0.5));
    let material = materials.add(Color::srgb(1.0, 0.9, 0.0));

    for _n in 1..config.count {
        commands.spawn((
            PbrBundle {
                mesh: mesh.clone(),
                material: material.clone(),
                transform: Transform::from_translation(generate_random_position_on_map(
                    config.size,
                    &mut global_rng,
                    2.0,
                )),
                ..default()
            },
            RngComponent::new(),
            RobotTarget {},
        ));
    }
}

fn place_robots(
    config: Res<Configuration>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut global_rng: ResMut<GlobalRng>,
) {
    let mesh = meshes.add(Cuboid::from_size(Vec3 {
        x: 0.5,
        y: 2.0,
        z: 0.5,
    }));

    // let mesh = meshes.add(Mesh::from(shape::Box::new(0.5, 2.0, 0.5)));
    let material = materials.add(Color::srgb(0.37, 0.8, 1.0));

    for _n in 1..config.count {
        commands.spawn((
            PbrBundle {
                mesh: mesh.clone(),
                material: material.clone(),
                transform: Transform::from_translation(generate_random_position_on_map(
                    config.size,
                    &mut global_rng,
                    2.0,
                )),
                ..default()
            },
            RngComponent::new(),
            Robot {},
        ));
    }
}

fn robot_target_system(
    unattached: Query<(Entity, &Transform), (With<RobotTarget>, Without<AttachedToRobot>)>,
    empty_robots: Query<
        Entity,
        (
            With<Robot>,
            Without<Cooldown>,
            Without<CarryTarget>,
            Without<Children>,
            Without<DropTarget>,
        ),
    >,
    mut commands: Commands,
) {
    let mut items = unattached.iter();
    empty_robots.iter().for_each(|entity| {
        let Some((item, position)) = items.next() else {
            return; // no empty items, return
        };

        commands.entity(item).insert(AttachedToRobot {});
        commands.entity(entity).insert(CarryTarget {
            item,
            position: position.translation,
        });
    });
}

fn robot_move_to_carry_system(
    config: Res<Configuration>,
    mut robots: Query<(Entity, &mut Transform, &CarryTarget, &mut RngComponent)>,
    time: Res<Time>,
    commands: ParallelCommands,
) {
    robots
        .par_iter_mut()
        .for_each(|(entity, mut transform, target, mut rng)| {
            let distance_sq = target.position.distance_squared(transform.translation);
            if distance_sq < 0.1 {
                let position = generate_random_position_on_map_rng(config.size, &mut rng, 2.0);
                commands.command_scope(|mut commands| {
                    commands
                        .entity(entity)
                        .insert(DropTarget { position })
                        .remove::<CarryTarget>();
                    commands.entity(target.item).set_parent(entity).insert(
                        Transform::from_translation(Vec3 {
                            x: 0.0,
                            y: 2.0,
                            z: 0.0,
                        }),
                    );
                });
                return;
            }

            let direction = (target.position - transform.translation).normalize();
            transform.translation += direction * config.speed * time.delta_seconds();
        });
}

fn robot_move_to_drop_system(
    config: Res<Configuration>,
    mut robots: Query<(
        Entity,
        &mut Transform,
        &DropTarget,
        &Children,
        &mut RngComponent,
    )>,
    time: Res<Time>,
    commands: ParallelCommands,
) {
    robots
        .par_iter_mut()
        .for_each(|(entity, mut transform, target, children, mut rng)| {
            let distance_sq = target.position.distance_squared(transform.translation);
            if distance_sq < 0.1 {
                for &child in children.iter() {
                    let pos = generate_random_position_by_offset(
                        &mut rng,
                        2.0,
                        transform.translation,
                        2.0,
                    );
                    commands.command_scope(|mut commands| {
                        commands
                            .entity(child)
                            .remove_parent()
                            .insert(Transform::from_translation(pos))
                            .remove::<AttachedToRobot>();
                    });
                }

                let time_left = Duration::from_secs_f32(rng.f32() * 3.0);
                commands.command_scope(|mut commands| {
                    commands
                        .entity(entity)
                        .insert(Cooldown { time_left })
                        .remove::<DropTarget>();
                });

                return;
            }

            let direction = (target.position - transform.translation).normalize();
            transform.translation += direction * config.speed * time.delta_seconds();
        });
}

fn robot_cooldown_system(
    mut robots: Query<(Entity, &mut Cooldown)>,
    time: Res<Time>,
    commands: ParallelCommands,
) {
    robots.par_iter_mut().for_each(|(entity, mut cooldown)| {
        cooldown.time_left = cooldown
            .time_left
            .saturating_sub(Duration::from_secs_f32(time.delta_seconds()));
        if cooldown.time_left.is_zero() {
            commands.command_scope(|mut commands| {
                commands.entity(entity).remove::<Cooldown>();
            });
        }
    });
}

fn generate_random_position_on_map(size: f32, global_rng: &mut ResMut<GlobalRng>, y: f32) -> Vec3 {
    let x = (global_rng.f32() - 0.5) * size;
    let z = (global_rng.f32() - 0.5) * size;

    Vec3 { x, y, z }
}

fn generate_random_position_on_map_rng(size: f32, rng: &mut RngComponent, y: f32) -> Vec3 {
    let x = (rng.f32() - 0.5) * size;
    let z = (rng.f32() - 0.5) * size;

    Vec3 { x, y, z }
}

fn generate_random_position_by_offset(
    rng: &mut RngComponent,
    y: f32,
    pos: Vec3,
    offset: f32,
) -> Vec3 {
    let x = (rng.f32() - 0.5) * offset + pos.x;
    let z = (rng.f32() - 0.5) * offset + pos.z;

    Vec3 { x, y, z }
}
