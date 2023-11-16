use bevy::{prelude::*, window::PresentMode};
use bevy_screen_diagnostics::{ScreenDiagnosticsPlugin, ScreenFrameDiagnosticsPlugin};
// use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy_turborand::prelude::*;

use std::f32::consts::PI;

const COUNT: usize = 20000;
const SIZE: f32 = 100.0;
const MOVESPEED: f32 = 5.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                present_mode: PresentMode::Immediate,
                ..default()
            }),
            ..default()
        }))
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
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // plane base
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane::from_size(SIZE))),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
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
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 90.0, 90.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
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
    pub time_left: f32,
}

fn place_items(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut global_rng: ResMut<GlobalRng>,
) {
    let mesh = meshes.add(Mesh::from(shape::Cube { size: 0.5 }));
    let material = materials.add(Color::rgb(1.0, 0.9, 0.0).into());

    for _n in 1..COUNT {
        commands.spawn((
            PbrBundle {
                mesh: mesh.clone(),
                material: material.clone(),
                transform: Transform::from_translation(generate_random_position_on_map(
                    &mut global_rng,
                    2.0,
                )),
                ..default()
            },
            RobotTarget {},
        ));
    }
}

fn place_robots(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut global_rng: ResMut<GlobalRng>,
) {
    let mesh = meshes.add(Mesh::from(shape::Box::new(0.5, 2.0, 0.5)));
    let material = materials.add(Color::rgb(0.37, 0.8, 1.0).into());

    for _n in 1..COUNT {
        commands.spawn((
            PbrBundle {
                mesh: mesh.clone(),
                material: material.clone(),
                transform: Transform::from_translation(generate_random_position_on_map(
                    &mut global_rng,
                    2.0,
                )),
                ..default()
            },
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
    for entity in empty_robots.iter() {
        let Some((item, position)) = items.next() else {
            return; // no empty items, return
        };

        commands.entity(item).insert(AttachedToRobot {});
        commands.entity(entity).insert(CarryTarget {
            item,
            position: position.translation,
        });
    }
}

fn robot_move_to_carry_system(
    mut robots: Query<(Entity, &mut Transform, &CarryTarget)>,
    time: Res<Time>,
    mut commands: Commands,
    mut global_rng: ResMut<GlobalRng>,
) {
    for (entity, mut transform, target) in &mut robots {
        let distance_sq = target.position.distance_squared(transform.translation);
        if distance_sq < 0.1 {
            let position = generate_random_position_on_map(&mut global_rng, 2.0);
            commands
                .entity(entity)
                .insert(DropTarget { position })
                .remove::<CarryTarget>();
            commands
                .entity(target.item)
                .set_parent(entity)
                .insert(Transform::from_translation(Vec3 {
                    x: 0.0,
                    y: 2.0,
                    z: 0.0,
                }));
            continue;
        }

        let direction = (target.position - transform.translation).normalize();
        transform.translation += direction * MOVESPEED * time.delta_seconds();
    }
}

fn robot_move_to_drop_system(
    mut robots: Query<(Entity, &mut Transform, &DropTarget, &Children)>,
    time: Res<Time>,
    mut commands: Commands,
    mut global_rng: ResMut<GlobalRng>,
) {
    for (entity, mut transform, target, children) in &mut robots {
        let distance_sq = target.position.distance_squared(transform.translation);
        if distance_sq < 0.1 {
            for &child in children.iter() {
                let pos = generate_random_position_by_offset(
                    &mut global_rng,
                    2.0,
                    transform.translation,
                    2.0,
                );
                commands
                    .entity(child)
                    .remove_parent()
                    .insert(Transform::from_translation(pos))
                    .remove::<AttachedToRobot>();
            }

            let time_left = global_rng.f32() * 3.0;
            commands
                .entity(entity)
                .insert(Cooldown { time_left })
                .remove::<DropTarget>();

            continue;
        }

        let direction = (target.position - transform.translation).normalize();
        transform.translation += direction * MOVESPEED * time.delta_seconds();
    }
}

fn robot_cooldown_system(
    mut robots: Query<(Entity, &mut Cooldown)>,
    time: Res<Time>,
    mut commands: Commands,
) {
    for (entity, mut cooldown) in &mut robots {
        cooldown.time_left -= time.delta_seconds();
        if cooldown.time_left < 0.0 {
            commands.entity(entity).remove::<Cooldown>();
        }
    }
}

fn generate_random_position_on_map(global_rng: &mut ResMut<GlobalRng>, y: f32) -> Vec3 {
    let x = (global_rng.f32() - 0.5) * SIZE;
    let z = (global_rng.f32() - 0.5) * SIZE;

    Vec3 { x, y, z }
}

fn generate_random_position_by_offset(
    global_rng: &mut ResMut<GlobalRng>,
    y: f32,
    pos: Vec3,
    offset: f32,
) -> Vec3 {
    let x = (global_rng.f32() - 0.5) * offset + pos.x;
    let z = (global_rng.f32() - 0.5) * offset + pos.z;

    Vec3 { x, y, z }
}