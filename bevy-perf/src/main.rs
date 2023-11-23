use bevy::{
    core::TaskPoolThreadAssignmentPolicy,
    core_pipeline::prepass::{DeferredPrepass, DepthPrepass, MotionVectorPrepass, NormalPrepass},
    pbr::{ClusterConfig, DefaultOpaqueRendererMethod},
    prelude::*,
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
    tasks::available_parallelism,
    window::PresentMode,
};
use bevy_screen_diagnostics::{ScreenDiagnosticsPlugin, ScreenFrameDiagnosticsPlugin};
// use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy_turborand::prelude::*;

use std::{f32::consts::PI, time::Duration};

const COUNT: usize = 500000;
const SIZE: f32 = 100000.0;
const MOVESPEED: f32 = 100.0;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
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
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        present_mode: PresentMode::Immediate,
                        ..default()
                    }),
                    ..default()
                }),
        )
        .insert_resource(Msaa::Off)
        .insert_resource(DefaultOpaqueRendererMethod::forward())
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
    // camera
    commands.spawn(Camera2dBundle::new_with_far(1.0));
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
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut global_rng: ResMut<GlobalRng>,
) {
    let mesh: Mesh2dHandle = meshes.add(shape::Circle::new(10.0).into()).into();
    let material = materials.add(ColorMaterial::from(Color::YELLOW));

    for _n in 1..COUNT {
        commands.spawn((
            MaterialMesh2dBundle {
                mesh: mesh.clone(),
                material: material.clone(),
                transform: Transform::from_translation(generate_random_position_on_map(
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
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut global_rng: ResMut<GlobalRng>,
) {
    let mesh: Mesh2dHandle = meshes
        .add(shape::Quad::new(Vec2::new(20., 40.)).into())
        .into();
    let material = materials.add(ColorMaterial::from(Color::LIME_GREEN));

    for _n in 1..COUNT {
        commands.spawn((
            MaterialMesh2dBundle {
                mesh: mesh.clone(),
                material: material.clone(),
                transform: Transform::from_translation(generate_random_position_on_map(
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
    let _ = info_span!("robot_target_system").entered();
    let mut items = unattached.iter();
    empty_robots.for_each(|entity| {
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
    mut robots: Query<(Entity, &mut Transform, &CarryTarget, &mut RngComponent)>,
    time: Res<Time>,
    commands: ParallelCommands,
) {
    let _ = info_span!("robot_move_to_carry_system").entered();
    robots
        .par_iter_mut()
        .for_each(|(entity, mut transform, target, mut rng)| {
            let distance_sq = target.position.distance_squared(transform.translation);
            if distance_sq < 0.1 {
                let position = generate_random_position_on_map_rng(&mut rng, 2.0);
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
            transform.translation += direction * MOVESPEED * time.delta_seconds();
        });
}

fn robot_move_to_drop_system(
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
    let _ = info_span!("robot_move_to_drop_system").entered();
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
            transform.translation += direction * MOVESPEED * time.delta_seconds();
        });
}

fn robot_cooldown_system(
    mut robots: Query<(Entity, &mut Cooldown)>,
    time: Res<Time>,
    commands: ParallelCommands,
) {
    let _ = info_span!("robot_cooldown_system").entered();
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

fn generate_random_position_on_map(global_rng: &mut ResMut<GlobalRng>, y: f32) -> Vec3 {
    let x = (global_rng.f32() - 0.5) * SIZE;
    let y = (global_rng.f32() - 0.5) * SIZE;

    Vec3 { x, y, z: 0. }
}

fn generate_random_position_on_map_rng(rng: &mut RngComponent, y: f32) -> Vec3 {
    let x = (rng.f32() - 0.5) * SIZE;
    let y = (rng.f32() - 0.5) * SIZE;

    Vec3 { x, y, z: 0. }
}

fn generate_random_position_by_offset(
    rng: &mut RngComponent,
    y: f32,
    pos: Vec3,
    offset: f32,
) -> Vec3 {
    let x = (rng.f32() - 0.5) * offset + pos.x;
    let y = (rng.f32() - 0.5) * offset + pos.z;

    Vec3 { x, y, z: 0. }
}
