use bevy::{math::Vec3Swizzles, prelude::*};
use bevy_rapier2d::prelude::*;

pub struct Plugin;

#[derive(Component)]
struct Player;

#[derive(Component)]
struct SideEffectTrigger;

#[derive(Component)]
struct JumpEffect;

#[derive(Component)]
struct Side(Transform);

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup)
            .add_system(update_player_input)
            .add_system(update_camera)
            .add_system(update_sides)
            .add_event::<SideEvent>()
            .add_system(side_events)
            .add_system(jump_effect)
            .add_system(display_events);
    }
}
/* A system that displays the events. */
fn display_events(
    mut collision_events: EventReader<CollisionEvent>,
    mut contact_force_events: EventReader<ContactForceEvent>,
) {
    for collision_event in collision_events.iter() {
        println!("Received collision event: {:?}", collision_event);
    }

    for contact_force_event in contact_force_events.iter() {
        println!("Received contact force event: {:?}", contact_force_event);
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut rapier_config: ResMut<RapierConfiguration>,
) {
    rapier_config.gravity = Vec2::new(0.0, -30.0);
    commands.spawn({
        let mut bundle = Camera2dBundle::default();
        bundle.projection.scaling_mode = bevy::render::camera::ScalingMode::FixedVertical(10.0);
        bundle
    });
    let map: Vec<Vec<char>> = include_str!("level.txt")
        .lines()
        .map(|line| line.chars().collect())
        .collect();
    let w = map.iter().map(|row| row.len()).max().unwrap();
    let h = map.len();
    let map = |x: usize, y: usize| map[h - 1 - y].get(x).copied().unwrap_or(' ');
    let index = |x, y| (x + y * (w + 1)) as u32;
    let mut trimesh_indices = Vec::new();
    let mut player_location = None;
    #[allow(clippy::needless_range_loop)]
    for x in 0..w {
        for y in 0..h {
            match map(x, y) {
                '#' => trimesh_indices.extend([
                    [index(x, y), index(x + 1, y), index(x, y + 1)],
                    [index(x + 1, y + 1), index(x, y + 1), index(x + 1, y)],
                ]),
                'L' => trimesh_indices.push([index(x, y), index(x + 1, y), index(x + 1, y + 1)]),
                'R' => trimesh_indices.push([index(x, y), index(x + 1, y), index(x, y + 1)]),
                'S' => {
                    player_location = Some((x, y));
                }
                ' ' => {}
                _ => unreachable!(),
            }
        }
    }
    commands.spawn((
        Collider::trimesh(
            (0..h + 1)
                .flat_map(|y| (0..w + 1).map(move |x| Vec2::new(x as _, y as _)))
                .collect(),
            trimesh_indices,
        ),
        SideEffectTrigger,
    ));

    let player_size = 1.0;
    let player_radius = player_size / 2.0;
    let player_border_radius = player_radius * 0.5;
    let player = commands
        .spawn((
            Player,
            SpriteBundle {
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(player_size)),
                    ..default()
                },
                transform: {
                    let (x, y) = player_location.unwrap();
                    Transform::from_xyz(x as f32 + 0.5, y as f32 + 0.5, 0.0)
                },
                texture: asset_server.load("texture.png"),
                ..default()
            },
            RigidBody::Dynamic,
            Velocity::zero(),
            Friction::new(1.5),
            // Collider::round_cuboid(
            //     player_radius, // - player_border_radius,
            //     player_radius, // - player_border_radius,
            //     player_border_radius,
            // ),
            Collider::cuboid(player_radius, player_radius),
            ColliderMassProperties::Density(1.0),
            ExternalForce::default(),
        ))
        .id();
    let sensor_width = player_size * 0.1;
    commands.spawn((
        Collider::cuboid(player_radius, sensor_width),
        TransformBundle::IDENTITY,
        Sensor,
        ActiveEvents::COLLISION_EVENTS,
        Side(Transform::from_translation(Vec3::new(
            0.0,
            player_radius,
            0.0,
        ))),
    ));
}

fn update_sides(
    mut sides: Query<(&mut Transform, &Side), Without<Player>>,
    player: Query<&Transform, With<Player>>,
) {
    let Some(player) = player.iter().next() else { return };
    for (mut transform, side) in sides.iter_mut() {
        *transform = player.mul_transform(side.0);
    }
}

enum SideEvent {
    Started(Entity),
    Stopped(Entity),
}

fn side_events(
    sides: Query<(), With<Side>>,
    side_triggers: Query<(), With<SideEffectTrigger>>,
    mut collisions: EventReader<CollisionEvent>,
    mut events: EventWriter<SideEvent>,
) {
    let mut process = |a, b, f: fn(Entity) -> SideEvent| {
        let mut check = |a, b| {
            if !sides.contains(a) {
                return;
            }
            if !side_triggers.contains(b) {
                return;
            }
            events.send(f(a));
        };
        check(a, b);
        check(b, a);
    };
    for event in collisions.iter() {
        match *event {
            CollisionEvent::Started(a, b, _) => {
                info!("{a:?}, {b:?}");
                process(a, b, SideEvent::Started);
            }
            CollisionEvent::Stopped(a, b, _) => {
                process(a, b, SideEvent::Stopped);
            }
        }
    }
}

fn jump_effect(
    mut player: Query<(&Transform, &mut ExternalImpulse), With<Player>>,
    sides: Query<&Side, With<JumpEffect>>,
    mut events: EventReader<SideEvent>,
) {
    let Some((transform, mut impulse)) = player.iter_mut().next() else { return };
    for event in events.iter() {
        match *event {
            SideEvent::Started(side) => {
                let Ok(side) = sides.get(side) else { continue; };
                impulse.impulse += transform
                    .with_translation(Vec3::ZERO)
                    .mul_transform(side.0)
                    .transform_point(Vec3::ZERO)
                    .xy()
                    * 100.0;
            }
            SideEvent::Stopped(_) => {}
        }
    }
}

fn update_player_input(
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(&Velocity, &mut ExternalForce), With<Player>>,
) {
    for (vel, mut force) in query.iter_mut() {
        let mut target_dir = None::<f32>;
        if keyboard_input.any_pressed([KeyCode::A, KeyCode::Left]) {
            *target_dir.get_or_insert(0.0) += 1.0;
        }
        if keyboard_input.any_pressed([KeyCode::D, KeyCode::Right]) {
            *target_dir.get_or_insert(0.0) -= 1.0;
        }
        if let Some(dir) = target_dir {
            let target_angvel = dir * 2.0 * std::f32::consts::PI;
            force.torque = (target_angvel - vel.angvel) * 10.0;
        } else {
            force.torque = 0.0;
        }
    }
}

fn update_camera(
    mut camera: Query<&mut Transform, (With<Camera2d>, Without<Player>)>,
    player: Query<&Transform, With<Player>>,
) {
    let Some(mut camera) = camera.iter_mut().next() else { return };
    let Some(player) = player.iter().next() else { return };
    camera.translation = player.translation;
}
