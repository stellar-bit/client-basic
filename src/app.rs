use std::{path::Path, sync::mpsc};

use self::{
    background_star::BackgroundStar,
    particles::{ParticleSystem, ShrinkingCircle},
};

use super::*;

#[cfg(not(target_arch = "wasm32"))]
mod controller_ffi;
mod controller_select;

use controller_select::Controller;

mod camera;
use camera::Camera;

mod background_star;
mod particles;
mod sounds;

use ellipsoid::prelude::egui_file::FileDialog;
use sounds::SoundManager;

#[cfg(target_arch = "wasm32")]
mod controller_wasm;
#[cfg(target_arch = "wasm32")]
use controller_wasm::Controller;
use futures_util::FutureExt;
use log::warn;

use rodio::{OutputStream, OutputStreamHandle};

const FRIENDLY_COLOR: Color = Color::from_rgb(0. / 255., 186. / 255., 130. / 255.);
const ENEMY_COLOR: Color = Color::from_rgb(186. / 255., 0. / 255., 50. / 255.);

const SPACECRAFT_Z: f32 = 0.2;
const ASTEROID_Z: f32 = 0.3;
const STAR_BASE_Z: f32 = 0.4;
const PROJECTILE_Z: f32 = 0.1;
const BACKGROUND_Z: f32 = 0.9;
const PARTICLES_Z: f32 = 0.1;
const OUTLINE_Z: f32 = 0.7;

#[repr(u32)]
#[derive(Default, Clone, Copy, strum::Display, strum::EnumIter, Debug)]
#[strum(serialize_all = "snake_case")]
pub enum SpacecraftTextures {
    #[default]
    White,
    BlockComponent,
    BlockComponentWhite,
    LaserProjectile,
    MissileProjectile,
    LaserWeaponComponent,
    LaserWeaponComponentWhite,
    MissileWeaponComponent,
    RaptorEngineComponent,
    RaptorEngineComponentWhite,
    StarBase,
    StarryNight,
}

impl Into<u32> for SpacecraftTextures {
    fn into(self) -> u32 {
        self as u32
    }
}

impl Textures for SpacecraftTextures {}

type Txts = SpacecraftTextures;

pub trait Drawable {
    fn shape(&self) -> Shape<Txts>;
}

impl Drawable for Asteroid {
    fn shape(&self) -> Shape<Txts> {
        // let texture = match self.material {
        //     Material::Carbon => Txts::CarbonAsteroid,
        //     Material::Copper => Txts::CopperAsteroid,
        //     Material::Iron => Txts::IronAsteroid,
        //     Material::Silicates => Txts::SiliconAsteroid,
        //     Material::Nickel => Txts::NickelAsteroid,
        // };
        // Colors by GPT-4
        let color = Color::from_hex(match self.material {
            Material::Carbon => 0x3D3D3D, // A shade of grey, symbolizing carbon's color in its graphite form
            Material::Copper => 0xB87333, // A shade of copper, symbolizing copper's distinctive color
            Material::Iron => 0x43464B,   // A shade of dark gray, symbolizing iron's color
            Material::Silicates => 0x607D8B, // A shade of blue-grey, symbolizing the color of common silicate minerals
            Material::Nickel => 0x758A5C, // A shade of grayish-green, symbolizing the color of nickel
        });
        Shape::new(self.body.bounds.clone()).set_color(color)
    }
}

impl Drawable for StarBase {
    fn shape(&self) -> Shape<Txts> {
        Shape::new(self.body.bounds.clone()).set_color(Color::WHITE)
    }
}

impl Drawable for Projectile {
    fn shape(&self) -> Shape<Txts> {
        Shape::from_square_centered().set_texture(Txts::LaserProjectile)
    }
}

struct AppIntervals {
    cmds_sync: Interval,
    game_sync: Interval,
}

#[derive(Default)]
struct EguiFields {
    server_addr: String,
    computer_path: String,
    computer_file_dialog: Option<FileDialog>,
}

pub struct SpacecraftApp {
    pub graphics: Graphics<Txts>,
    pub network_msgs: Vec<ClientRequest>,
    network_connection: NetworkConnection,
    game: Arc<RwLock<Game>>,
    user: Arc<RwLock<User>>,
    time_intervals: AppIntervals,
    controller: Controller,
    follow_target: Option<GameObjectId>,
    mouse_position: Vec2,
    right_mouse_pressed: bool,
    camera: Camera,
    particle_system: ParticleSystem,
    egui_fields: EguiFields,
    sound_manager: SoundManager,
}

impl App<Txts> for SpacecraftApp {
    async fn new(window: winit::window::Window) -> Self {
        let graphics = Graphics::new(window).await;
        let game: Arc<RwLock<Game>> = Arc::new(RwLock::new(Game::new()));
        let user: Arc<RwLock<User>> = Arc::new(RwLock::new(User::Spectator));

        let server_addr = std::env::var("SERVER_ADDR").unwrap_or("ws://0.0.0.0:39453".to_string());

        let (_audio_stream, audio_stream_handle) = OutputStream::try_default().unwrap();

        let mut network_connection =
            NetworkConnection::start(server_addr, game.clone(), user.clone())
                .await
                .unwrap();

        network_connection
            .send(ClientRequest::Join(rand::random(), rand::random()))
            .await
            .unwrap();
        network_connection
            .send(ClientRequest::FullGameSync)
            .await
            .unwrap();

        Self {
            game,
            user,
            network_connection,
            time_intervals: AppIntervals {
                cmds_sync: Interval::new(time::Duration::from_millis(300)),
                game_sync: Interval::new(time::Duration::from_millis(3000)),
            },
            controller: Controller::new(),
            follow_target: None,
            mouse_position: vec2(0.0, 0.0),
            camera: Camera::new(-10.),
            particle_system: ParticleSystem::new(),
            graphics,
            network_msgs: vec![],
            right_mouse_pressed: false,
            egui_fields: EguiFields::default(),
            sound_manager: SoundManager::new(),
        }
    }

    fn graphics(&self) -> &Graphics<Txts> {
        &self.graphics
    }

    fn graphics_mut(&mut self) -> &mut Graphics<Txts> {
        &mut self.graphics
    }

    fn update(&mut self, dt: f32) {
        self.update_main(dt);

        self.update_camera();

        self.update_particles(dt);

        self.update_network();

        self.process_events();
    }

    fn draw(&mut self) {
        let camera_gtransform =
            GTransform::from_inflation(self.camera.mp()).translate(-self.camera.position());

        self.draw_background();
        self.draw_game_objects(camera_gtransform);
        self.draw_particles(camera_gtransform);
        self.draw_egui();
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        if let WindowEvent::MouseWheel { delta, .. } = event {
            match delta {
                winit::event::MouseScrollDelta::LineDelta(_, y) => {
                    self.camera.zoom += y * 0.1;
                }
                winit::event::MouseScrollDelta::PixelDelta(pos) => {
                    self.camera.zoom += pos.y as f32 / 1000.;
                }
            }

            return true;
        } else if let WindowEvent::MouseInput { state, button, .. } = event {
            let mouse_real_pos = self.camera.to_real(self.mouse_position);

            match button {
                winit::event::MouseButton::Left => {
                    if state != &winit::event::ElementState::Pressed {
                        return false;
                    }
                    let game = self.game.read().unwrap();
                    for (id, game_object) in game.game_objects.iter() {
                        if game_object.collides_point(mouse_real_pos) {
                            self.follow_target = Some(*id);
                            self.camera.offset = Vec2::ZERO;
                            break;
                        }
                    }
                }
                winit::event::MouseButton::Right => {
                    self.right_mouse_pressed = state == &winit::event::ElementState::Pressed;
                }
                _ => {}
            }
        } else if let WindowEvent::CursorMoved { position, .. } = event {
            let x = position.x as f32 / self.graphics.window().inner_size().width as f32;
            let y = position.y as f32 / self.graphics.window().inner_size().height as f32;

            let last_mouse_pos = self.mouse_position;
            self.mouse_position = vec2(x, -y) * 2. - vec2(1., -1.);

            let delta_mouse_pos = self.mouse_position - last_mouse_pos;

            if self.right_mouse_pressed {
                self.camera.offset -= delta_mouse_pos / self.camera.mp();
            }
        }
        false
    }
}

impl SpacecraftApp {
    async fn connect(&mut self, server_addr: String) {}

    fn update_main(&mut self, _dt: f32) {
        let user = self.user();
        let mut game = self.game.write().unwrap();

        if game.sync.last_update >= now() {
            warn!(
                "Last game update is in the future [+{:?}]??!",
                game.sync.last_update - now()
            );
            return;
        }
        let game_dt = now() - game.sync.last_update;
        game.update(game_dt.as_secs_f32());

        if self.time_intervals.cmds_sync.check() {
            self.network_msgs.push(ClientRequest::GameCmdsSync);
        }
        if self.time_intervals.game_sync.check() {
            self.network_msgs.push(ClientRequest::FullGameSync);
        }

        let network_game_cmds =
            self.controller
                .retrieve_cmds(&mut game, &user, &self.graphics.egui_platform.context());
        if !network_game_cmds.is_empty() {
            self.network_msgs
                .push(ClientRequest::ExecuteGameCmds(network_game_cmds));
        }
    }

    fn update_camera(&mut self) {
        let game = self.game.read().unwrap();

        if let Some(follow_target) = self.follow_target {
            if let Some(game_object) = game.game_objects.get(&follow_target) {
                self.camera.center = game_object.body().position;
            } else {
                self.follow_target = None;
                self.camera.offset += self.camera.center;
                self.camera.center = Vec2::ZERO;
            }
        } else {
            self.camera.center = Vec2::ZERO;
        }
    }

    fn update_particles(&mut self, dt: f32) {
        self.particle_system.update(dt);
    }

    fn update_network(&mut self) {
        let requests = std::mem::take(&mut self.network_msgs);
        if !requests.is_empty() {
            self.network_connection.send_multiple(requests).unwrap();
        }
    }

    fn process_events(&mut self) {
        let events = std::mem::take(&mut self.game.write().unwrap().events);

        let mut sound_sources = vec![];
        for event in events {
            sound_sources.push(match event {
                GameEvent::ProjectileLaunched(projectile) => {
                    Some((projectile.body.position, "laser_fired"))
                }
                GameEvent::SpacecraftDeployed(spacecraft) => {
                    Some((spacecraft.body.position, "spacecraft_deployed"))
                }
                GameEvent::GameObjectDestroyed(destroyed, destroyer) => match destroyer {
                    GameObject::StarBase(_) => Some((destroyer.body().position, "star_base_hit")),
                    _ => match destroyed {
                        GameObject::Projectile(projectile) => {
                            Some((projectile.body.position, "laser_hit"))
                        }
                        _ => None,
                    },
                },
            });
        }
        // dbg!(self.camera.mp());
        // dbg!(self.camera.mp().log2());
        // dbg!(self.camera.zoom);
        for (position, sound) in sound_sources.into_iter().filter_map(|x| x) {
            let distance = (position - self.camera.position()).length();
            // let volume = ((1. - distance / 1000.) - 1. / self.camera.mp().sqrt()/15.
            //     + 0.1)
            //     .max(0.)
            //     .min(1.);

            let zoom_eff = 1. / (-self.camera.mp().log2());
            let dist_eff = (1. - distance.powi(2) / 100_000. / (-self.camera.mp().log2()));
            let volume = zoom_eff * dist_eff;
            let volume = volume.max(0.).min(1.);
            self.sound_manager.play(sound, volume);
        }
    }

    fn draw_background(&mut self) {
        let background = Shape::from_square_centered()
            .apply(GTransform::from_inflation(2.))
            .set_color(Color::BLACK)
            .set_z(BACKGROUND_Z);

        self.graphics.add_geometry(background.into());

        // for star in &self.stars {
        //     let pos = star.pos - self.camera.position() * star.parallax;
        //     let size_mp = self.camera.mp().powf(1000.*star.parallax);
        //     let star_gt = GTransform::from_translation(pos).inflate(star.radius * size_mp);
        //     let star_shape = Shape::from_circle(5).apply(star_gt).set_color(Color::from_rgba(0.9, 0.9, 1., 0.1)).set_z(BACKGROUND_Z-0.001);

        //     self.graphics.add_geometry(star_shape.into());
        // }
    }

    fn draw_game_objects(&mut self, camera_gtransform: GTransform) {
        let user = self.user();
        let game = self.game.read().unwrap();

        for asteroid in game.asteroids() {
            let gtransform = camera_gtransform
                .translate(asteroid.body.position)
                .rotate(asteroid.body.rotation);

            self.graphics
                .add_geometry(asteroid.shape().apply(gtransform).set_z(ASTEROID_Z).into());
            self.graphics.add_geometry(
                asteroid
                    .shape()
                    .apply(gtransform.inflate_fixed(0.1 * self.camera.mp()))
                    .set_color(Color::from_rgb(0.05, 0.05, 0.05))
                    .reset_texture()
                    .set_z(OUTLINE_Z)
                    .into(),
            );
        }
        for spacecraft in game.spacecrafts() {
            let spacecraft_gtransform = camera_gtransform
                .translate(spacecraft.body.position)
                .rotate(spacecraft.body.rotation)
                .translate(-spacecraft.center_of_mass);

            let outline_color = if let User::Player(id) = user {
                if id == spacecraft.owner {
                    FRIENDLY_COLOR
                } else {
                    ENEMY_COLOR
                }
            } else {
                ENEMY_COLOR
            };

            for component in spacecraft.components.values() {
                let rotation = match component {
                    Component::Weapon(weapon) => weapon.rotation,
                    _ => 0.,
                };

                let outline_thickness = 0.1;

                let gtransform = spacecraft_gtransform
                    .translate(component.body().position.as_vec2())
                    .rotate(component.body().orientation.to_radians() + rotation)
                    .translate(-Vec2::ONE * 0.5)
                    .stretch(component.body().scale().as_vec2());

                let outline_gtransform = spacecraft_gtransform
                    .translate(component.body().position.as_vec2())
                    .rotate(component.body().orientation.to_radians() + rotation)
                    .translate(-Vec2::ONE * (0.5 + outline_thickness / 2.))
                    .stretch(component.body().scale().as_vec2() + outline_thickness);

                let texture = match component.body().origin {
                    ComponentType::LaserWeapon => SpacecraftTextures::LaserWeaponComponent,
                    ComponentType::Central => SpacecraftTextures::BlockComponent,
                    ComponentType::MissileLauncher => SpacecraftTextures::MissileWeaponComponent,
                    ComponentType::RaptorEngine => SpacecraftTextures::RaptorEngineComponent,
                    ComponentType::SteelBlock => SpacecraftTextures::BlockComponent,
                };

                let component_shape = Shape::from_square()
                    .apply(gtransform)
                    .set_texture(texture)
                    .set_z(if component.body().top().is_none() {
                        SPACECRAFT_Z
                    } else {
                        SPACECRAFT_Z - 0.01
                    });

                self.graphics.add_geometry(component_shape.into());
                if component.body().top().is_none() {
                    let component_outline = Shape::from_square()
                        .apply(outline_gtransform)
                        .set_color(outline_color)
                        .set_z(OUTLINE_Z)
                        .reset_texture();
                    self.graphics.add_geometry(component_outline.into());
                }
            }
        }
        for star_base in game.star_bases() {
            let gtransform = camera_gtransform.translate(star_base.body.position);

            let outline_color = if let User::Player(id) = user {
                if id == star_base.owner {
                    FRIENDLY_COLOR
                } else {
                    ENEMY_COLOR
                }
            } else {
                ENEMY_COLOR
            };

            self.graphics.add_geometry(
                star_base
                    .shape()
                    .apply(gtransform)
                    .set_z(STAR_BASE_Z)
                    .into(),
            );
            self.graphics.add_geometry(
                star_base
                    .shape()
                    .apply(gtransform.inflate_fixed(0.05 * self.camera.mp()))
                    .set_z(OUTLINE_Z)
                    .reset_texture()
                    .set_color(outline_color)
                    .into(),
            )
        }

        for projectile in game.projectiles() {
            let gtransform = camera_gtransform
                .translate(projectile.body.position)
                .rotate(projectile.body.rotation)
                .stretch(projectile.size);

            let shape = projectile.shape().set_z(PROJECTILE_Z).apply(gtransform);

            self.graphics.add_geometry(shape.into());
        }

        // for game_object in game.game_objects.values() {
        //     let collider = game_object.body().bounds.clone();
        //     let gtransform = GTransform::from_translation(game_object.body().position).rotate(game_object.body().rotation);

        //     self.graphics.add_geometry(Shape::new(collider).apply(gtransform).apply(camera_gtransform).set_color(Color::GREEN).into());
        // }
    }

    fn draw_particles(&mut self, camera_gtransform: GTransform) {
        let game = self.game.read().unwrap();
        for spacecraft in game.spacecrafts() {
            for component in spacecraft.components.values() {
                let component_gtransform = GTransform::from_translation(spacecraft.body.position)
                    .rotate(spacecraft.body.rotation)
                    .translate(component.body().position.as_vec2() - spacecraft.center_of_mass)
                    .rotate(component.body().orientation.to_radians());
                match component {
                    Component::Engine(engine) => {
                        if !engine.active || engine.fuel <= 0. {
                            continue;
                        }
                        let radius = engine.power * 0.6;
                        let position = component_gtransform.translate(engine.ignition_point).center;
                        self.particle_system
                            .add_particle(Box::new(ShrinkingCircle::new(
                                position,
                                spacecraft.body.velocity
                                    - Vec2::from_angle(
                                        engine.body.orientation.to_radians()
                                            + spacecraft.body.rotation,
                                    ) * engine.power
                                        * (rand::random::<f32>() * 2. + 1.)
                                    + Vec2::random_unit_circle() * 0.5,
                                radius,
                                rand::random::<f32>() * 2.7,
                                Color::from_rgb(0.5, 0.5, 1.),
                                Color::from_rgb(0.05, 0.05, 0.2),
                            )));
                    }
                    _ => (),
                }
            }
        }

        for particle_shape in self.particle_system.draw() {
            self.graphics.add_geometry(
                particle_shape
                    .apply(camera_gtransform)
                    .set_z(PARTICLES_Z)
                    .into(),
            );
        }
    }

    fn draw_egui(&mut self) {
        let user = self.user();
        let game = self.game.read().unwrap();

        if let Some(follow_target) = self.follow_target {
            let game_object = game.game_objects.get(&follow_target).unwrap();
            let game_object_text: &'static str = (game_object.clone()).into();
            egui::Window::new(format!("{} [{}]", game_object_text, follow_target)).show(
                &self.graphics.egui_platform.context(),
                |ui| {
                    ui.label(format!("Position: {:?}", game_object.body().position));
                    ui.label(format!("Velocity: {:?}", game_object.body().velocity));
                    ui.label(format!("Rotation: {:?}", game_object.body().rotation));
                    ui.label(format!(
                        "Angular velocity: {:?}",
                        game_object.body().angular_velocity
                    ));
                    ui.label(format!("Health: {}", game_object.health()));
                    ui.label(format!("Owner: {:?}", game_object.owner()));
                    // ui.collapsing("Details", |ui| {
                    match game_object {
                        GameObject::Asteroid(asteroid) => {
                            ui.label(format!("Radius: {}", asteroid.radius));
                            ui.label(format!("Material: {:?}", asteroid.material));
                        }
                        GameObject::Spacecraft(spacecraft) => {
                            ui.label(format!("Tags: {:?}", spacecraft.tags));
                            ui.label(format!("Components count: {}", spacecraft.components.len()));
                            ui.label(format!("Center of mass: {:?}", spacecraft.center_of_mass));
                            // for component in spacecraft.components.values() {
                            //     ui.label(format!("Component: {:?}", component));
                            // }
                        }
                        GameObject::StarBase(star_base) => {
                            ui.label(format!("Hangars count: {}", star_base.hangars.len()));
                            ui.label("-------------Hangars-------------");
                            for (i, hangar) in star_base.hangars.iter().enumerate() {
                                ui.label(format!("{i}: {hangar}"));
                            }
                        }
                        _ => (),
                    }
                    // });
                },
            );
        }

        egui::Window::new("Player info").show(&self.graphics.egui_platform.context(), |ui| {
            match user {
                User::Server => (),
                User::Player(player) => {
                    ui.label(format!("Materials: {:?}", game.players[&player].materials));
                    let mut game_objects =
                        game.game_objects.clone().into_iter().collect::<Vec<_>>();
                    game_objects.sort_by_key(|x| x.0);
                    for (id, game_object) in game_objects {
                        if game_object.owner() == Some(player) {
                            let game_object_text: &'static str = game_object.into();
                            if ui
                                .button(format!("{} [{}]", game_object_text, id))
                                .clicked()
                            {
                                self.follow_target = Some(id);
                                self.camera.offset = Vec2::ZERO;
                            }
                        }
                    }
                }
                User::Spectator => {
                    ui.label("Spectator");
                }
            }
        });

        egui::Window::new("Server log").show(&self.graphics.egui_platform.context(), |ui| {
            for msg in &game.log {
                ui.label(msg);
            }
        });

        egui::Window::new("Controller").show(&self.graphics.egui_platform.context(), |ui| {
            if ui.button("Load").clicked() {
                let mut dialog = FileDialog::open_file(self.controller.computer_path());
                dialog.open();
                self.egui_fields.computer_file_dialog = Some(dialog);
            }
            if let Some(dialog) = &mut self.egui_fields.computer_file_dialog {
                if dialog
                    .show(&self.graphics.egui_platform.context())
                    .selected()
                {
                    if let Some(file) = dialog.path() {
                        let computer_path = file.as_os_str().to_str().unwrap().into();
                        self.controller.select_computer(computer_path);
                    }
                }
            }

            ui.label(format!(
                "Active computer: {}",
                self.controller
                    .computer_path()
                    .unwrap_or("None".into())
                    .as_os_str()
                    .to_str()
                    .unwrap()
            ));
        });

        drop(game);
    }

    fn user(&self) -> User {
        match *self.user.read().unwrap() {
            User::Player(player_id) => {
                if self.game.read().unwrap().players.contains_key(&player_id) {
                    User::Player(player_id)
                } else {
                    User::Spectator
                }
            }
            user => user,
        }
    }
}
