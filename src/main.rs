use dioxus::prelude::*;
use futures::stream;
use futures_util::StreamExt;
use kalosm_sound::{
    AsyncSourceFromStream, DenoisedExt, TranscribeChunkedAudioStreamExt, VoiceActivityStreamExt,
    WhisperBuilder, WhisperSource,
};
use web_sys::wasm_bindgen::{JsCast, JsValue, prelude::Closure};

use crate::mic::{AudioData, stream_microphone};

mod mic;

fn main() {
    launch(|| {
        let messages = use_signal(Vec::new);
        use_future(move || async move {
            start_web_sys_audio_stream(messages).await;
        });
        rsx! {
            for message in messages.iter() {
                div {
                    "{message}"
                }
            }
        }
    });
}

async fn start_web_sys_audio_stream(mut messages: Signal<Vec<String>>) {
    let (sender, mut receiver) = futures::channel::mpsc::unbounded();

    let mut sender = sender.clone();
    let on_array_buffer: Closure<dyn FnMut(JsValue)> =
        Closure::new(move |array_buffer: JsValue| {
            let array_buffer: AudioData = serde_wasm_bindgen::from_value(array_buffer).unwrap();
            _ = sender.start_send(array_buffer);
        });
    stream_microphone(on_array_buffer.as_ref().unchecked_ref(), None);
    on_array_buffer.forget();

    // Create a new small whisper model
    let model = WhisperBuilder::default()
        .with_source(WhisperSource::tiny_en())
        .build()
        .await
        .unwrap();

    let first = receiver.next().await.unwrap();
    let sample_rate = first.sample_rate;
    let audio = AsyncSourceFromStream::new(
        receiver.flat_map(|content| stream::iter(content.samples)),
        sample_rate,
    );

    let mut stream = audio
        .denoise_and_detect_voice_activity()
        .inspect(|audio| tracing::info!("probability: {:?}", audio.probability))
        .rechunk_voice_activity()
        .with_end_threshold(0.01)
        .transcribe(model);
    while let Some(text) = stream.next().await {
        if text.probability_of_no_speech() < 0.1 {
            messages.push(text.text().into());
        }
    }
}
