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
    pub fn new(audio: Audio, _cx: &mut Context<Self>) -> Self {
        let stream_handle = get_output_stream_handle();
        let sink = Sink::try_new(&stream_handle).unwrap();

        AudioPlayer {
            audio,
            sink: Arc::new(Mutex::new(sink)),
            playing: false,
            progress: 0.0,
        }
    }

    /// Polls playback position on a foreground timer instead of the old
    /// monitor thread + cushy Dynamic; ends itself once playback stops
    /// (paused or finished). Started from `toggle` each time playback
    /// begins or resumes, not from `new`: a task started once in `new`
    /// would see the sink empty on its very first tick (nothing has been
    /// appended yet) and exit immediately, so progress would never update
    /// and `playing` would stay true forever after the first play.
    fn start_progress_polling(&mut self, cx: &mut Context<Self>) {
        let sink_for_task = self.sink.clone();
        let total = self.audio.length_in_sec();
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
                    // `Progress::value` expects a percentage in 0.0..100.0,
                    // not the 0.0..1.0 fraction computed above.
                    this.progress = elapsed_fraction * 100.0;
                    cx.notify();
                    !playing
                })
                .unwrap_or(true);
            if should_stop {
                break;
            }
        })
        .detach();
    }

    fn toggle(&mut self, cx: &mut Context<Self>) {
        // Lock through a cloned `Arc` handle, not `self.sink` directly, so
        // the `MutexGuard`'s lifetime isn't tied to `self` — otherwise it
        // would still be considered borrowed when `self.start_progress_polling`
        // (which needs `&mut self`) is called below.
        let sink_handle = self.sink.clone();
        let sink = sink_handle.lock().unwrap();
        let just_started_or_resumed = if sink.empty() {
            drop(sink);
            let sink = sink_handle.lock().unwrap();
            sink.append(AudioWrapper {
                audio: self.audio.clone(),
                num_sample: 0,
            });
            sink.play();
            self.playing = true;
            true
        } else if self.playing {
            sink.pause();
            self.playing = false;
            false
        } else {
            sink.play();
            self.playing = true;
            true
        };
        if just_started_or_resumed {
            self.start_progress_polling(cx);
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
