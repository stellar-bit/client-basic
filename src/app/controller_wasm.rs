use super::*;
use js_sys::{Function, Object, Reflect, WebAssembly};
use log::warn;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};

pub struct Controller {
    network_game_cmds: Vec<GameCmd>,
}

impl Controller {
    pub fn new() -> Self {
        Self {
            network_game_cmds: vec![],
        }
    }
}

pub static mut EXECUTION_FUNCTION: Option<Function> = None;

impl Controller {
    pub fn retrieve_cmds(
        &mut self,
        game: &mut Game,
        user: &User,
        _egui_context: &egui::Context,
    ) -> Vec<GameCmd> {
        let Some(execution_func) = (unsafe { &EXECUTION_FUNCTION }) else {
            return vec![];
        };

        let game_raw = serialize_bytes(game).unwrap();
        let user_raw = serialize_bytes(user).unwrap();

        let game_raw = unsafe { js_sys::Uint8Array::new(&js_sys::Uint8Array::view(&game_raw)) };
        let user_raw = unsafe { js_sys::Uint8Array::new(&js_sys::Uint8Array::view(&user_raw)) };

        let cmds = execution_func
            .call2(
                &JsValue::NULL,
                &JsValue::from(&game_raw),
                &JsValue::from(user_raw),
            )
            .unwrap_throw();

        let cmds_raw = cmds
            .dyn_into::<js_sys::Uint8Array>()
            .unwrap_throw()
            .to_vec();
        let cmds = deserialize_bytes::<Vec<GameCmd>>(&cmds_raw).unwrap_throw();

        for cmd in cmds.clone() {
            if let Err(err) = game.execute_cmd(*user, cmd) {
                error!("Error executing command: {:?}", err);
            };
        }

        cmds
    }
}

#[wasm_bindgen]
pub async unsafe fn set_computer_func(execute_func: Function) {
    EXECUTION_FUNCTION = Some(execute_func);
    warn!("Setting computer function");
}
