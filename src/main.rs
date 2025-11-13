use std::collections::HashSet;

use dioxus::prelude::*;
use dioxus_primitives::slider::SliderValue;
use futures::stream;
use futures_util::StreamExt;
use kalosm_sound::{
    AsyncSource, AsyncSourceFromStream, AsyncSourceTranscribeExt, Segment, Whisper, WhisperBuilder,
    WhisperSource,
};
use strum::Display;
use web_sys::wasm_bindgen::{JsCast, JsValue, prelude::Closure};

use crate::{
    components::{progress::*, select::*, slider::*, toggle_group::*},
    mic::{AudioData, StreamOptions, stream_microphone},
};

mod components;
mod mic;

fn main() {
    launch(app);
}

fn app() -> Element {
    let model = use_signal(|| None);
    let mut from_display = use_signal(|| false);
    let chunks = use_store(Vec::new);
    let mut speech_threshold = use_signal(|| 0.9);
    let loading_progress = use_signal(|| 0.0);

    let whisper = use_resource(move || async move {
        match model() {
            Some(model) => match load_model(model, loading_progress).await {
                Ok(model) => model,
                Err(err) => {
                    tracing::error!("Error loading model: {}", err);
                    std::future::pending().await
                }
            },
            None => std::future::pending().await,
        }
    });

    use_resource(move || async move {
        if let Some(whisper) = whisper() {
            if let Err(err) = start_web_sys_audio_stream(from_display(), chunks, whisper).await {
                tracing::error!("Error starting audio stream: {}", err);
            }
        }
    });

    rsx! {
        document::Stylesheet {
            href: asset!("/assets/dx-components-theme.css")
        }

        div {
            width: "100vw",
            height: "100vh",
            display: "flex",
            flex_direction: "column",
            align_items: "center",
            gap: "1rem",

            div {
                padding_top: "0.5rem",
                display: "flex",
                flex_direction: "column",
                align_items: "center",
                gap: "0.5rem",

                "Source"
                ToggleGroup {
                    horizontal: true,
                    allow_multiple_pressed: false,
                    on_pressed_change: move |value: HashSet<_>| from_display.set(value.contains(&1)),
                    ToggleItem { index: 0usize,
                        "Mic"
                    }
                    ToggleItem { index: 1usize,
                        "Device"
                    }
                }
            }
            div {
                padding_top: "0.5rem",
                display: "flex",
                flex_direction: "column",
                align_items: "center",
                gap: "0.5rem",

                "Speech threshold ({speech_threshold:.2})"
                Slider {
                    label: "Speech threshold",
                    horizontal: true,
                    min: 0.8,
                    max: 1.0,
                    step: 0.001,
                    default_value: SliderValue::Single(0.9),
                    on_value_change: move |value: SliderValue| {
                        // Extract the f64 value from SliderValue::Single
                        let SliderValue::Single(v) = value;
                        speech_threshold.set(v);
                    },
                    SliderTrack {
                        SliderRange {}
                        SliderThumb {}
                    }
                }
            }
            div {
                padding_top: "0.5rem",
                display: "flex",
                flex_direction: "column",
                align_items: "center",
                gap: "0.5rem",

                "Model"
                ModelSelector { model }

                if model.read().is_some() && loading_progress() < 1.0 {
                    "Loading..."
                    Progress {
                        value: loading_progress() as f64,
                        max: 1.0,
                        ProgressIndicator {}
                    }
                }
            }

            div {
                width: "100vw",
                height: "100vh",
                display: "flex",
                flex_direction: "column",
                align_items: "center",
                justify_content: "center",
                gap: "2rem",

                Recording {
                    speech_threshold,
                    chunks
                }
            }
        }
    }
}

struct EditableSegment {
    original: Segment,
    text: String,
}

impl From<Segment> for EditableSegment {
    fn from(segment: Segment) -> Self {
        EditableSegment {
            text: segment.text().to_string(),
            original: segment,
        }
    }
}

#[component]
fn Recording(speech_threshold: ReadSignal<f64>, chunks: Store<Vec<EditableSegment>>) -> Element {
    rsx! {
        div {
            width: "70vw",
            for chunk in chunks.iter() {
                Chunk {
                    speech_threshold,
                    chunk
                }
            }
        }
    }
}

#[component]
fn Chunk(speech_threshold: ReadSignal<f64>, chunk: Store<EditableSegment>) -> Element {
    let current_chunk = chunk.read();
    if 1.0 - current_chunk.original.probability_of_no_speech() > speech_threshold() {
        return VNode::empty();
    }
    let text = current_chunk.text.as_str();
    let mut editing = use_signal(|| false);
    rsx! {
        if editing() {
            input {
                class: "chunk-input",
                value: text,
                oninput: move |event| {
                    let new_text = event.value();
                    chunk.write().text = new_text;
                },
                onblur: move |_| editing.set(false),
            }
        } else {
            div {
                ondoubleclick: move |_| editing.set(true),
                {text}
            }
        }
    }
}

#[component]
fn ModelSelector(model: WriteSignal<Option<ModelSource>>) -> Element {
    let sources = ModelSource::ALL.iter().enumerate().map(|(i, f)| {
        rsx! {
            SelectOption::<ModelSource> { index: i, value: *f, text_value: "{f}",
                "{f}"
                SelectItemIndicator {}
            }
        }
    });

    rsx! {
        Select::<ModelSource> { placeholder: "Select a model...",
            on_value_change: move |value| model.set(value),
            SelectTrigger { aria_label: "Select Trigger", width: "12rem", SelectValue {} }
            SelectList { aria_label: "Select Demo",
                {sources}
            }
        }
    }
}

#[derive(Copy, Clone, Display, PartialEq)]
enum ModelSource {
    #[strum(to_string = "Tiny")]
    Tiny,
    #[strum(to_string = "Tiny English")]
    TinyEn,
    #[strum(to_string = "Base")]
    Base,
    #[strum(to_string = "Base English")]
    BaseEn,
    #[strum(to_string = "Medium")]
    Medium,
    #[strum(to_string = "Medium English")]
    MediumEn,
    #[strum(to_string = "Large V3")]
    LargeV3,
    #[strum(to_string = "Distiled Medium English")]
    DistilMediumEn,
    #[strum(to_string = "Distiled Large V3.5")]
    DistilLargeV3_5,
    #[strum(to_string = "Distiled Large V3")]
    DistilLargeV3,
    #[strum(to_string = "Large V3 Turbo")]
    LargeV3Turbo,
}

impl ModelSource {
    const ALL: &[Self] = &[
        ModelSource::Tiny,
        ModelSource::TinyEn,
        ModelSource::Base,
        ModelSource::BaseEn,
        ModelSource::Medium,
        ModelSource::MediumEn,
        ModelSource::LargeV3,
        ModelSource::DistilMediumEn,
        ModelSource::DistilLargeV3_5,
        ModelSource::DistilLargeV3,
        ModelSource::LargeV3Turbo,
    ];

    fn source(self) -> WhisperSource {
        match self {
            ModelSource::Tiny => WhisperSource::tiny(),
            ModelSource::TinyEn => WhisperSource::tiny_en(),
            ModelSource::Base => WhisperSource::base(),
            ModelSource::BaseEn => WhisperSource::base_en(),
            ModelSource::Medium => WhisperSource::medium(),
            ModelSource::MediumEn => WhisperSource::medium_en(),
            ModelSource::LargeV3 => WhisperSource::large_v3(),
            ModelSource::DistilMediumEn => WhisperSource::distil_medium_en(),
            ModelSource::DistilLargeV3_5 => WhisperSource::distil_large_v3_5(),
            ModelSource::DistilLargeV3 => WhisperSource::distil_large_v3(),
            ModelSource::LargeV3Turbo => WhisperSource::large_v3_turbo(),
        }
    }
}

async fn start_recording(from_display: bool) -> Option<impl AsyncSource + Unpin> {
    let (sender, mut receiver) = futures::channel::mpsc::unbounded();

    let mut sender = sender.clone();
    let on_array_buffer: Closure<dyn FnMut(JsValue)> =
        Closure::new(move |array_buffer: JsValue| {
            if let Ok(array_buffer) = serde_wasm_bindgen::from_value::<AudioData>(array_buffer) {
                _ = sender.start_send(array_buffer);
            }
        });
    stream_microphone(
        on_array_buffer.as_ref().unchecked_ref(),
        Some(StreamOptions::new().with_from_display(from_display)),
    )
    .ok()?;
    on_array_buffer.forget();

    let first = receiver.next().await?;
    let sample_rate = first.sample_rate;
    Some(AsyncSourceFromStream::new(
        receiver.flat_map(|content| stream::iter(content.samples)),
        sample_rate,
    ))
}

async fn load_model(
    model: ModelSource,
    mut loading_progress: Signal<f32>,
) -> dioxus::Result<Whisper> {
    let source = model.source();
    Ok(WhisperBuilder::default()
        .with_source(source)
        .build_with_loading_handler(move |progress| loading_progress.set(progress.progress()))
        .await?)
}

async fn start_web_sys_audio_stream(
    from_display: bool,
    mut chunks: Store<Vec<EditableSegment>>,
    model: Whisper,
) -> dioxus::Result<()> {
    let Some(audio) = start_recording(from_display).await else {
        return Ok(());
    };

    let mut stream = audio.transcribe(model);
    while let Some(text) = stream.next().await {
        chunks.push(text.into());
    }

    Ok(())
}
