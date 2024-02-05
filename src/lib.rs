#![feature(let_chains)]
#![feature(extract_if)]

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use ellipsoid::prelude::*;
use stellar_bit_core::prelude::{vec2, Vec2, *};

mod app;
pub use app::{SpacecraftApp, SpacecraftTextures};

mod network;
use network::NetworkConnection;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    ellipsoid::run::<SpacecraftTextures, SpacecraftApp>().await;
}
