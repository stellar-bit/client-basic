use super::*;
use log::warn;
use wasm_bindgen_futures::JsFuture;
use web_sys::{ErrorEvent, MessageEvent, WebSocket};

pub struct WebNetworkClient {
    ws: WebSocket,
    time_delay: Arc<RwLock<i64>>,
}

impl WebNetworkClient {
    pub async fn connect(
        server_addr: &str,
        game: Arc<RwLock<Game>>,
        user: Arc<RwLock<User>>,
    ) -> Result<Self, NetworkError> {
        warn!("Connecting to server at {}", server_addr);
        let promise = js_sys::Promise::new(&mut |resolve, reject| {
            let ws = WebSocket::new(server_addr).unwrap();
            let cloned_ws = ws.clone();

            let on_open_callback = Closure::wrap(Box::new(move || {
                resolve.call1(&JsValue::NULL, &cloned_ws);
            }) as Box<dyn Fn()>);

            let on_error_callback = Closure::wrap(Box::new(move |event: ErrorEvent| {
                reject.call1(&JsValue::NULL, &event);
            }) as Box<dyn Fn(ErrorEvent)>);

            ws.set_onopen(Some(on_open_callback.as_ref().unchecked_ref()));
            ws.set_onerror(Some(on_error_callback.as_ref().unchecked_ref()));

            on_open_callback.forget();
            on_error_callback.forget();
        });

        let ws = JsFuture::from(promise)
            .await
            .map(|value| value.dyn_into::<WebSocket>().unwrap())
            .map_err(|value| value.dyn_into::<ErrorEvent>().unwrap())
            .unwrap();

        let time_delay = Arc::new(RwLock::new(0));
        let time_delay_clone = time_delay.clone();
        let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
            if let Ok(abuf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
            } else if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                warn!("Received text");
            } else if let Ok(blob) = e.data().dyn_into::<web_sys::Blob>() {
                // better alternative to juggling with FileReader is to use https://crates.io/crates/gloo-file
                let fr = web_sys::FileReader::new().unwrap();
                let fr_c = fr.clone();
                // create onLoadEnd callback
                let game = game.clone();
                let time_delay = time_delay_clone.clone();
                let user = user.clone();
                let onloadend_cb =
                    Closure::<dyn FnMut(_)>::new(move |_e: web_sys::ProgressEvent| {
                        let array = js_sys::Uint8Array::new(&fr_c.result().unwrap());
                        let response = deserialize_bytes(&array.to_vec()).unwrap();
                        handle_server_response(
                            response,
                            game.clone(),
                            time_delay.clone(),
                            user.clone(),
                        );
                    });
                fr.set_onloadend(Some(onloadend_cb.as_ref().unchecked_ref()));
                fr.read_as_array_buffer(&blob).expect("blob not readable");
                onloadend_cb.forget();
            } else {
                warn!("Received unknown message type");
            }
        });

        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        // forget the callback to keep it alive
        onmessage_callback.forget();
        Ok(Self { ws, time_delay })
    }

    pub fn send(&self, msg: ClientRequest) -> Result<(), NetworkError> {
        let msg_raw = serialize_bytes(&msg).map_err(|_| NetworkError::IncorrectDataFormat)?;
        self.ws.send_with_u8_array(&msg_raw).unwrap();
        Ok(())
    }

    pub fn send_multiple(&self, msgs: Vec<ClientRequest>) -> Result<(), NetworkError> {
        for msg in msgs {
            self.send(msg)?;
        }
        Ok(())
    }
}
