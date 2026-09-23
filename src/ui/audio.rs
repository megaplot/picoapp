use std::cell::RefCell;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gpui_kit::component::button::Button;
use gpui_kit::component::progress::Progress;
use gpui_kit::{div, Context, IntoElement, ParentElement, Render, Styled, Window};
use rodio::{OutputStream, OutputStreamHandle, Sink};

use crate::outputs::Audio;

thread_local! {
    static STREAM: RefCell<Option<(OutputStream, OutputStreamHandle)>> = RefCell::new(None);
}

fn get_output_stream_handle() -> OutputStreamHandle {
    STREAM.with_borrow_mut(|stream_tup| {
        if let Some((_stream, stream_handle)) = stream_tup {
            stream_handle.clone()
        } else {
            let (stream, stream_handle) = OutputStream::try_default().unwrap();
            *stream_tup = Some((stream, stream_handle.clone()));
            stream_handle
        }
    })
}

#[derive(Clone, Debug)]
struct AudioWrapper {
    audio: Audio,
    num_sample: usize,
}

impl Iterator for AudioWrapper {
    type Item = f32;

    #[inline]
    fn next(&mut self) -> Option<f32> {
        let output = if self.num_sample < self.audio.data.len() {
            Some(self.audio.data[self.num_sample])
        } else {
            None
        };
        self.num_sample = self.num_sample.wrapping_add(1);
        output
    }
}

impl rodio::Source for AudioWrapper {
    #[inline]
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    #[inline]
    fn channels(&self) -> u16 {
        1
    }
    #[inline]
    fn sample_rate(&self) -> u32 {
        self.audio.sr
    }
    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

pub struct AudioPlayer {
    audio: Audio,
    sink: Arc<Mutex<Sink>>,
    playing: bool,
    progress: f32,
}

impl AudioPlayer {
    pub fn new(audio: Audio, cx: &mut Context<Self>) -> Self {
        let stream_handle = get_output_stream_handle();
        let sink = Sink::try_new(&stream_handle).unwrap();
        let sink = Arc::new(Mutex::new(sink));

        // Poll progress on a foreground timer instead of the old monitor
        // thread + cushy Dynamic; ends itself once playback stops.
        let sink_for_task = sink.clone();
        let total = audio.length_in_sec();
        cx.spawn(async move |this, cx| loop {
            cx.background_executor()
                .timer(Duration::from_millis(16))
                .await;
            let (playing, elapsed_fraction) = {
                let sink = sink_for_task.lock().unwrap();
                if sink.empty() {
                    (false, 0.0)
                } else {
                    let pos = sink.get_pos().as_secs_f32();
                    (true, (pos / total.max(1e-6)).clamp(0.0, 1.0))
                }
            };
            let should_stop = this
                .update(cx, |this, cx| {
                    this.playing = playing;
                    this.progress = elapsed_fraction;
                    cx.notify();
                    !playing
                })
                .unwrap_or(true);
            if should_stop {
                break;
            }
        })
        .detach();

        AudioPlayer {
            audio,
            sink,
            playing: false,
            progress: 0.0,
        }
    }

    fn toggle(&mut self, cx: &mut Context<Self>) {
        let sink = self.sink.lock().unwrap();
        if sink.empty() {
            drop(sink);
            let sink = self.sink.lock().unwrap();
            sink.append(AudioWrapper {
                audio: self.audio.clone(),
                num_sample: 0,
            });
            sink.play();
            self.playing = true;
        } else if self.playing {
            sink.pause();
            self.playing = false;
        } else {
            sink.play();
            self.playing = true;
        }
        cx.notify();
    }
}

impl Render for AudioPlayer {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let label = if self.playing { "Pause" } else { "Play" };
        div()
            .flex()
            .flex_row()
            .gap_2()
            .items_center()
            .child(
                Button::new("audio-toggle")
                    .label(label)
                    .on_click(cx.listener(|this, _, _window, cx| this.toggle(cx))),
            )
            .child(Progress::new("audio-progress").value(self.progress))
    }
}
