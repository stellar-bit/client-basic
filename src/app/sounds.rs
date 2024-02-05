use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::{collections::HashMap, io::Result};

pub struct SoundManager {
    stream: OutputStream,
    stream_handle: OutputStreamHandle,
    sound_cache: HashMap<String, Vec<u8>>,
}

impl SoundManager {
    pub fn new() -> Self {
        let (stream, stream_handle) = OutputStream::try_default().unwrap();
        let sound_cache = HashMap::new();

        Self {
            stream,
            stream_handle,
            sound_cache,
        }
    }

    pub fn load_sound(&mut self, sound: &str) -> Result<()> {
        let file_path = format!("assets/sounds/{}.wav", sound);
        let bytes = std::fs::read(file_path)?;
        self.sound_cache.insert(sound.to_string(), bytes);
        Ok(())
    }

    pub fn play(&mut self, sound: &str, volume: f32) {
        if !self.sound_cache.contains_key(sound) {
            self.load_sound(sound).unwrap();
        }

        let sound_data = self.sound_cache.get(sound).unwrap();
        let cursor = std::io::Cursor::new(sound_data.clone());
        let source = Decoder::new(cursor).unwrap();

        let sink = Sink::try_new(&self.stream_handle).unwrap();
        sink.set_volume(volume);
        sink.append(source);
        sink.detach();
    }
}
