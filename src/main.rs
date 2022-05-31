use benimator::{AnimationPlugin, Play, SpriteSheetAnimation};
use bevy::prelude::*;
use bevy::sprite::collide_aabb::{collide, Collision};
use std::{ops::Deref, time::Duration};

#[derive(Component)]
struct Movement {
    location: Vec3,
    velocity: Vec3,
    accel: Vec3,
    is_left: bool,
    speed_scale: f32,
    gravity: Vec3,
    forces: Vec3,
}

impl Movement {
    fn max_force(&mut self) {
        if self.forces.y > 20.0 {
            self.forces.y = 20.0;
        }
    }
    fn apply_force(&mut self, force: f32) {
        self.forces.y += force; 
    }
}

#[derive(Default)]
struct CollisionEvent;

#[derive(Component)]
struct Floor {}

#[derive(Component)]
struct BrickShort {}

#[derive(Component, PartialEq, Eq)]
enum Collider {
    Solid,
    Push,
}

fn main() {
    App::new()
        .init_resource::<Animations>()
        .add_plugins(DefaultPlugins)
        .add_plugin(AnimationPlugin::default())
        .add_startup_system_to_stage(StartupStage::PreStartup, setup_animations)
        .add_startup_system(initial_setup)
        .add_event::<CollisionEvent>()
        .add_system_set(
            SystemSet::new()
                .label("handle")
                .after("move")
                .with_system(input_handling)
        )
        .add_system_set(SystemSet::new().label("move").with_system(movement_system))
        .add_system_set(
            SystemSet::new()
                .label("apply")
                .before("move")
                .with_system(apply_gravity)
                .with_system(apply_jump)
                .with_system(jump_handling)
        )
        .add_system_set(
            SystemSet::new()
                .label("collisions")
                .after("apply")
                .before("move")
                .with_system(floor_player_collision_system.label("floor"))
                .with_system(brick_short_collision)
                .with_system(box_collision_system),
        )
        .add_system(animate_sprite_system)
        .run();
}

#[derive(Default)]
struct Animations {
    idle: Handle<SpriteSheetAnimation>,
    moving: Handle<SpriteSheetAnimation>,
}

fn setup_animations(
    mut handles: ResMut<Animations>,
    mut assets: ResMut<Assets<SpriteSheetAnimation>>,
) {
    handles.idle = assets.add(SpriteSheetAnimation::from_range(
        0..=3,
        Duration::from_millis(150),
    ));
    handles.moving = assets.add(SpriteSheetAnimation::from_range(
        6..=9,
        Duration::from_millis(150),
    ));
}

fn initial_setup(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut animations: ResMut<Assets<SpriteSheetAnimation>>,
    anim: Res<Animations>,
) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    let texture_handle = server.load("hero.png");
    let texture_atlas = TextureAtlas::from_grid(texture_handle, Vec2::new(16.0, 16.0), 6, 5);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);
    commands
        .spawn_bundle(SpriteSheetBundle {
            texture_atlas: texture_atlas_handle,
            transform: Transform::from_scale(Vec3::splat(2.5)),
            ..Default::default()
        })
        .insert(Movement {
            location: Vec3::from_slice(&[100., 100., 0.]),
            velocity: Vec3::ZERO,
            accel: Vec3::ZERO,
            is_left: false,
            speed_scale: 125.0,
            gravity: Vec3::new(0.0, -1.5, 0.0),
            forces: Vec3::ZERO,
        })
        .insert(anim.idle.clone())
        .insert(Play);

    let box_handle = server.load("mbox.png");
    commands
        .spawn_bundle(SpriteBundle {
            texture: box_handle,
            ..Default::default()
        })
        .insert(Collider::Push)
        .insert(Movement {
            velocity: Vec3::new(0.0, -1.0, 0.0),
            location: Vec3::ZERO,
            accel: Vec3::ZERO,
            is_left: false,
            speed_scale: 70.0,
            gravity: Vec3::new(0.0, -1.5, 0.0),
            forces: Vec3::ZERO,
        });

    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new(1280., 42.)),
                ..Default::default()
            },
            texture: server.load("floor_base.png"),
            transform: Transform {
                translation: Vec3::new(0.0, -300., 0.0),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Collider::Solid)
        .insert(Floor {});

    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new(100., 42.)),
                ..Default::default()
            },
            texture: server.load("floor_short.png"),
            transform: Transform {
                translation: Vec3::new(0.0, -200., 0.0),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Collider::Solid)
        .insert(BrickShort {});

    commands
        .spawn_bundle(SpriteBundle {
            sprite: Sprite {
                custom_size: Some(Vec2::new(100., 42.)),
                ..Default::default()
            },
            texture: server.load("floor_short.png"),
            transform: Transform {
                translation: Vec3::new(150.0, -100., 0.0),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Collider::Solid)
        .insert(BrickShort {});
}

fn input_handling(
    keys: Res<Input<KeyCode>>,
    mut move_q: Query<&mut Movement, With<TextureAtlasSprite>>,
) {
    let mut movement = move_q.single_mut();

    movement.velocity = Vec3::ZERO;

    for key in keys.get_pressed() {
        movement.velocity += match key {
            KeyCode::W => Vec3::new(0.0, 1.0, 0.0),
            KeyCode::A => Vec3::new(-1.0, 0.0, 0.0),
            KeyCode::S => Vec3::new(0.0, -1.0, 0.0),
            KeyCode::D => Vec3::new(1.0, 0.0, 0.0),
            _ => Vec3::ZERO,
        };
    }
}

fn jump_handling(
    keys: Res<Input<KeyCode>>,
    mut player_q: Query<(&Transform, &TextureAtlasSprite, &mut Movement), Without<Collider>>,
brick_q: Query<(&Transform, &Sprite), With<BrickShort>>,
) {
    let (transform, sprite, mut movement) = player_q.single_mut();
    let size = sprite.custom_size.unwrap_or(Vec2::new(41.6, 51.2));
    for (brick_transform, brick_sprite) in brick_q.iter() {
        let collision = collide(
            transform.translation,
            size,
            brick_transform.translation,
            brick_sprite.custom_size.unwrap_or(Vec2::ZERO),
        );
        let brick_offset_x = brick_sprite.custom_size.unwrap_or(Vec2::ZERO).x / 2.0;
        let brick_offset_y = brick_sprite.custom_size.unwrap_or(Vec2::ZERO).y / 2.0;
        let sprite_offset_y = size.y / 2.0;

        if let Some(Collision::Top) = collision {
            if keys.just_pressed(KeyCode::Space){
                println!("jump");
                movement.accel.y += 20.;
            }
        } else if brick_transform.translation.x - brick_offset_x > transform.translation.x
            || brick_transform.translation.x + brick_offset_x < transform.translation.x
        {
            movement.gravity.y = -1.5;
        }
    }

}

fn animate_sprite_system(
    animations: Res<Animations>,
    mut move_q: Query<&mut Movement, With<TextureAtlasSprite>>,
    mut query: Query<&mut Handle<SpriteSheetAnimation>>,
    mut sprite_q: Query<&mut TextureAtlasSprite>,
) {
    let mut movement = move_q.single_mut();
    let mut animation = query.single_mut();

    if movement.velocity.x < -0.1 {
        movement.is_left = true;
    } else if movement.velocity.x > 0.1 {
        movement.is_left = false;
    }

    let sprite_atlas = sprite_q.get_single_mut();
    match sprite_atlas {
        Ok(mut x) => x.flip_x = movement.is_left,
        Err(_) => println!("Oh no! Couldn't find hero sprite"),
    }

    if movement.velocity.eq(&Vec3::ZERO) {
        *animation = animations.idle.clone();
    } else {
        *animation = animations.moving.clone();
    }
}

fn movement_system(
    mut moveable_q: Query<(&mut Movement, &mut Transform)>,
    time: Res<Time>,
) {
    for (mut movement, mut transform) in moveable_q.iter_mut() {
        if movement.velocity != Vec3::ZERO {
            movement.velocity = movement.velocity.normalize();
        }
        let velocity = movement.velocity + movement.accel;
        let speed_scale = movement.speed_scale;
        movement.location += velocity * speed_scale * time.delta_seconds();

        transform.translation = movement.location;

        movement.accel = Vec3::ZERO;
    }
}

fn apply_gravity(mut moveable_q: Query<&mut Movement>,
            ) {
    for mut movement in moveable_q.iter_mut() {
        let gravity = movement.gravity;
        movement.accel += gravity;
    }
}
fn apply_jump(mut moveable_q: Query<&mut Movement>,
    keys: Res<Input<KeyCode>>,
            ) {
    for mut movement in moveable_q.iter_mut() {
        if keys.pressed(KeyCode::Space) {
                movement.apply_force(6.0);
        }
        movement.max_force();
        let forces = movement.forces;
        movement.accel += forces;
        movement.forces = Vec3::ZERO;
    }
}

fn box_collision_system(
    mut player_q: Query<(&Transform, &TextureAtlasSprite, &mut Movement), Without<Collider>>,
    mut collider_q: Query<(&mut Movement, &Transform, &Sprite, &Collider), With<Movement>>,
) {
    let (player_transform, player_sprite, mut player_movement) = player_q.single_mut();
    let player_size = player_sprite.custom_size.unwrap_or(Vec2::new(41.6, 51.2));

    for (mut movement, transform, sprite, collider) in collider_q.iter_mut() {
        let collision = collide(
            player_transform.translation,
            player_size,
            transform.translation,
            sprite.custom_size.unwrap_or(Vec2::new(38.0, 38.0)),
        );
        if let Collider::Push = collider {
            if let Some(collision) = collision {
                match collision {
                    Collision::Left => {
                        if player_movement.velocity.x > 0.0 {
                            player_movement.speed_scale = 70.0;
                            movement.velocity.x = 1.0;
                        } else {
                            player_movement.speed_scale = 155.0;
                            movement.velocity.x = 0.0;
                        }
                    }
                    Collision::Right => {
                        if player_movement.velocity.x < 0.0 {
                            player_movement.speed_scale = 70.0;
                            movement.velocity.x = -1.0;
                        } else {
                            player_movement.speed_scale = 155.0;
                            movement.velocity.x = 0.0;
                        }
                    }
                    Collision::Top => {
                        if player_movement.velocity.y < 0.0 {
                            player_movement.speed_scale = 70.0;
                            movement.velocity.y = -1.0
                        } else {
                            player_movement.speed_scale = 155.0;
                            movement.velocity.y = 0.0;
                        }
                    }
                    Collision::Bottom => {
                        if player_movement.velocity.y > 0.0 {
                            player_movement.speed_scale = 70.0;
                            movement.velocity.y = 1.0;
                        } else {
                            player_movement.speed_scale = 155.0;
                            movement.velocity.y = 0.0;
                        }
                    }
                    Collision::Inside => {}
                };
            } else {
                player_movement.speed_scale = 155.0;
                movement.velocity = Vec3::ZERO;
            }
        }
    }
}

fn floor_player_collision_system(
    mut player_q: Query<(&Transform, &TextureAtlasSprite, &mut Movement), Without<Collider>>,
    brick_q: Query<(&Transform, &Sprite), With<Floor>>,
) {
    let (transform, sprite, mut movement) = player_q.single_mut();
    let size = sprite.custom_size.unwrap_or(Vec2::new(41.6, 51.2));
    for (brick_transform, brick_sprite) in brick_q.iter() {
        let collision = collide(
            transform.translation,
            size,
            brick_transform.translation,
            brick_sprite.custom_size.unwrap_or(Vec2::ZERO),
        );
        let brick_offset = brick_sprite.custom_size.unwrap_or(Vec2::ZERO).x / 2.0;
        //        println!("brick_offset = {}", brick_offset);
        if let Some(Collision::Top) = collision {
            movement.accel.y = 0.;
        } else if brick_transform.translation.x - brick_offset > transform.translation.x
            || brick_transform.translation.x + brick_offset < transform.translation.x
        {
            //           println!("gravity on ");
            movement.gravity = Vec3::new(0.0, -1.5, 0.0);
        }
    }
}

fn brick_short_collision(
    mut player_q: Query<(&Transform, &TextureAtlasSprite, &mut Movement), Without<Collider>>,
    brick_q: Query<(&Transform, &Sprite), With<BrickShort>>,
    keys: Res<Input<KeyCode>>, 
) {
    let (transform, sprite, mut movement) = player_q.single_mut();
    let size = sprite.custom_size.unwrap_or(Vec2::new(41.6, 51.2));
    for (brick_transform, brick_sprite) in brick_q.iter() {
        let collision = collide(
            transform.translation,
            size,
            brick_transform.translation,
            brick_sprite.custom_size.unwrap_or(Vec2::ZERO),
        );
        let brick_offset_x = brick_sprite.custom_size.unwrap_or(Vec2::ZERO).x / 2.0;
        let brick_offset_y = brick_sprite.custom_size.unwrap_or(Vec2::ZERO).y / 2.0;
        let sprite_offset_y = size.y / 2.0;

        if let Some(Collision::Top) = collision {
            movement.accel.y = 0.;
            
        } else if brick_transform.translation.x - brick_offset_x > transform.translation.x
            || brick_transform.translation.x + brick_offset_x < transform.translation.x
        {
            movement.gravity.y = -1.5;
        }
    }
}

/*fn collision(
    mut collision_events: EventReader<CollisionEvent>,
    mut player_q: Query<(&Transform, &TextureAtlasSprite, &mut Movement), Without<Collider>>,
    ) {
    let (_transform, _sprite, mut movement) = player_q.single_mut();
    if collision_events.iter().count() > 0 {
        movement.gravity.y = 0.0;
    }

}
*/
