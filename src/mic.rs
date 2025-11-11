use js_sys::Function;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Audio data received from the microphone stream callback
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AudioData {
    pub(crate) samples: Vec<f32>,
    #[serde(rename = "sampleRate")]
    pub(crate) sample_rate: u32,
}

/// Options for configuring the microphone stream
#[wasm_bindgen]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamOptions {
    #[serde(rename = "bufferSize")]
    buffer_size: u32,
}

impl Default for StreamOptions {
    fn default() -> Self {
        Self { buffer_size: 4096 }
    }
}

impl StreamOptions {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the buffer size (must be power of 2, typically 2048)
    pub fn set_buffer_size(&mut self, size: u32) {
        self.buffer_size = size;
    }

    /// Get the buffer size
    pub fn buffer_size(&self) -> u32 {
        self.buffer_size
    }
}

/// Import the JavaScript streamMicrophone function
#[wasm_bindgen(module = "/src/stream.js")]
extern "C" {
    #[wasm_bindgen(js_name = streamMicrophone)]
    fn stream_microphone_js(callback: &Function, options: JsValue);
}

pub fn stream_microphone(function: &js_sys::Function, options: Option<StreamOptions>) {
    let opts = options.unwrap_or_default();
    let opts_js = serde_wasm_bindgen::to_value(&opts).unwrap();

    stream_microphone_js(function, opts_js)
}
