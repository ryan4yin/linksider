use bevy::{math::Vec3Swizzles, prelude::*};
use bevy_rapier2d::prelude::*;

use crate::game::{
    side::{self, powerup, Blank, Powerup, Side, SideActivateEvent},
    DisableRotationControl, PlayerInput,
};

pub fn init(app: &mut App) {
    app.add_system(effect)
        .add_system(powerup)
        .add_system(effect_toggle);
}

#[derive(Component)]
pub struct Effect;

fn effect_toggle(
    sides: Query<(&Side, Option<&Handle<AudioSink>>), With<Effect>>,
    mut events: EventReader<SideActivateEvent>,
    audio: Res<Audio>,
    asset_server: Res<AssetServer>,
    audio_sinks: Res<Assets<AudioSink>>,
    mut commands: Commands,
) {
    for event in events.iter() {
        let Ok((side, audio_sink)) = sides.get(event.side()) else { continue };
        // let Ok(mut parent) = parents.get_mut(side.parent) else { continue };
        match event {
            SideActivateEvent::Activated(_) => {
                if let Some(sink) = audio_sink.and_then(|sink| audio_sinks.get(sink)) {
                    sink.stop();
                }
                commands.entity(event.side()).insert(
                    audio_sinks.get_handle(audio.play_with_settings(
                        asset_server.load("slide.ogg"),
                        PlaybackSettings::LOOP,
                    )),
                );
                commands.entity(side.parent).insert(DisableRotationControl);
            }
            SideActivateEvent::Deactivated(_) => {
                if let Some(sink) = audio_sink.and_then(|sink| audio_sinks.get(sink)) {
                    sink.stop();
                }
                commands.entity(event.side()).remove::<Handle<AudioSink>>();
                commands
                    .entity(side.parent)
                    .remove::<DisableRotationControl>();
            }
        };
    }
}

fn effect(
    time: Res<Time>,
    mut parents: Query<(Option<&PlayerInput>, &Transform, &mut Velocity)>,
    sides: Query<(&Side, &Handle<AudioSink>), (With<side::Active>, With<Effect>)>,
    audio_sinks: Res<Assets<AudioSink>>,
) {
    for (side, audio_sink) in sides.iter() {
        let Ok((input, transform, mut velocity)) = parents.get_mut(side.parent) else { continue };
        let direction = transform
            .with_translation(Vec3::ZERO)
            .mul_transform(side.transform)
            .transform_point(Vec3::ZERO)
            .xy();
        velocity.linvel += direction * time.delta_seconds() * 10.0;
        if let Some(input) = input {
            let move_direction = direction.rotate(Vec2::new(0.0, 1.0));
            velocity.linvel += move_direction * time.delta_seconds() * input.0 * 100.0;

            if let Some(sink) = audio_sinks.get(audio_sink) {
                sink.set_volume(Vec2::dot(velocity.linvel, move_direction).abs().min(1.0));
            }
        }
    }
}

fn powerup(
    mut commands: Commands,
    sides: Query<&Side, With<Blank>>,
    powerups: Query<(With<Powerup>, With<Effect>)>,
    mut events: EventReader<powerup::Event>,

    asset_server: Res<AssetServer>,
) {
    for event in events.iter() {
        let Ok(side) = sides.get(event.side) else { continue };
        if !powerups.contains(event.powerup) {
            continue;
        }
        commands.entity(event.powerup).despawn();
        commands.entity(event.side).insert(Effect).remove::<Blank>();

        commands
            .entity(event.side)
            .insert(Collider::cuboid(0.4, 0.1));
        commands.entity(side.parent).insert(Friction::new(0.0));

        // TODO: different system?
        commands.entity(event.side).insert((
            Sprite {
                custom_size: Some(Vec2::new(1.0, 0.25)),
                ..default()
            },
            asset_server.load::<Image, _>("side_effects/slide.png"),
            Visibility::default(),
            ComputedVisibility::default(),
        ));
    }
}
