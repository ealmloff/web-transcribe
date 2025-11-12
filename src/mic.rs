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
    #[serde(rename = "fromDisplay")]
    from_display: bool,
}

impl Default for StreamOptions {
    fn default() -> Self {
        Self {
            buffer_size: 4096,
            from_display: false,
        }
    }
}

impl StreamOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_buffer_size(mut self, size: u32) -> Self {
        self.buffer_size = size;
        self
    }

    pub fn with_from_display(mut self, from_display: bool) -> Self {
        self.from_display = from_display;
        self
    }
}

/// Import the JavaScript streamMicrophone function
#[wasm_bindgen(module = "/src/stream.js")]
extern "C" {
    #[wasm_bindgen(js_name = streamMicrophone, catch)]
    fn stream_microphone_js(callback: &Function, options: JsValue) -> Result<(), JsValue>;
}

pub fn stream_microphone(
    function: &js_sys::Function,
    options: Option<StreamOptions>,
) -> Result<(), JsValue> {
    let opts = options.unwrap_or_default();
    let opts_js = serde_wasm_bindgen::to_value(&opts)?;

    stream_microphone_js(function, opts_js)
}
