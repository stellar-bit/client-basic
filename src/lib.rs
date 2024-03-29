#![feature(let_chains)]
#![feature(extract_if)]

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use ellipsoid::prelude::*;
use stellar_bit_core::prelude::{vec2, Vec2, *};
use std::sync::{Arc,RwLock,Mutex};

mod app;
pub use app::{SpacecraftApp, Txts};

mod network;
use network::NetworkConnection;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn run() {
    ellipsoid::run::<Txts, SpacecraftApp>();
}
