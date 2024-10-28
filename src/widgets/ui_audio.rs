use std::cell::RefCell;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cushy::animation::ZeroToOne;
use cushy::figures::units::Px;
use cushy::figures::Zero;
use cushy::value::Dynamic;
use cushy::value::{Destination, Source};
use cushy::widget::{MakeWidget, Widget};
use cushy::widgets::progress::{Progress, Progressable};
use rodio::{OutputStream, OutputStreamHandle, Sink};

use crate::outputs::Audio;

thread_local! {
    // Note that the `OutputStream` must be kept alive as long as the `Sink` is
    // in use. Since an `OutputStream` is not `Send`, it doesn't really make
    // sense to hold an `(OutputStream, Sink)` below, because we wouldn't be
    // able to use it from the monitoring thread. It's probably best to share
    // the output stream via thread-local storage.
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
pub struct AudioWrapper {
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

    #[inline]
    fn try_seek(&mut self, _: Duration) -> Result<(), rodio::source::SeekError> {
        // TBD how to handle it (since we don't seek for now, it should not matter).
        Ok(())
    }
}

enum Status {
    Stopped,
    Paused,
    Playing(ZeroToOne),
}

#[derive(Clone)]
struct Player {
    sink: Arc<Mutex<Sink>>,
    audio_wrapper: AudioWrapper,
}

impl Player {
    pub fn new(stream_handle: &OutputStreamHandle, audio: Audio) -> Self {
        let sink = Sink::try_new(&stream_handle).unwrap();
        Self {
            sink: Arc::new(Mutex::new(sink)),
            audio_wrapper: AudioWrapper {
                num_sample: 0,
                audio,
            },
        }
    }

    pub fn initialize_playback(&self) {
        let sink = self.sink.lock().unwrap();
        sink.append(self.audio_wrapper.clone());
    }

    pub fn pause(&self) {
        let sink = self.sink.lock().unwrap();
        sink.pause();
    }

    pub fn unpause(&self) {
        let sink = self.sink.lock().unwrap();
        sink.play();
    }

    pub fn status(&self) -> Status {
        let sink = self.sink.lock().unwrap();
        if sink.empty() {
            Status::Stopped
        } else if sink.is_paused() {
            Status::Paused
        } else {
            let pos = sink.get_pos();
            Status::Playing(ZeroToOne::new(
                pos.as_secs_f32() / self.audio_wrapper.audio.length_in_sec(),
            ))
        }
    }

    pub fn monitor_progress(&self, progress: &Dynamic<Progress>, is_playing: &Dynamic<bool>) {
        loop {
            let status = self.status();
            match status {
                Status::Playing(percent) => {
                    progress.set(Progress::Percent(percent));
                    std::thread::sleep(Duration::from_millis(10));
                }
                _ => {
                    break;
                }
            }
        }

        let status = self.status();
        match status {
            Status::Stopped => {
                is_playing.set(false);
                progress.set(Progress::Percent(ZeroToOne::ZERO));
            }
            Status::Paused => {
                is_playing.set(false);
            }
            _ => {}
        }
    }
}

pub fn audio_player_widget(audio: &Audio) -> impl Widget {
    let progress = Dynamic::new(Progress::Percent(ZeroToOne::ZERO));
    let is_playing = Dynamic::new(false);

    // TODO: Avoid clone=
    let player = Player::new(&get_output_stream_handle(), audio.clone());

    is_playing
        .map_each(|is_playing| if *is_playing { "⏸" } else { "▶" })
        .into_button()
        .on_click({
            let player = player.clone();
            let progress = progress.clone();
            let is_playing = is_playing.clone();
            move |_| {
                let spawn_monitor_thread = match player.status() {
                    Status::Stopped => {
                        player.initialize_playback();
                        is_playing.set(true);
                        true
                    }
                    Status::Paused => {
                        player.unpause();
                        is_playing.set(true);
                        true
                    }
                    _ => {
                        player.pause();
                        false
                    }
                };
                if spawn_monitor_thread {
                    let player = player.clone();
                    let progress = progress.clone();
                    let is_playing = is_playing.clone();
                    std::thread::spawn(move || player.monitor_progress(&progress, &is_playing));
                }
            }
        })
        .centered()
        .and(
            progress
                .clone()
                .progress_bar()
                .width(Px::new(100)) // Why is it not possible to use `Px::new(100)..`
                .centered(),
        )
        .into_columns()
        .contain()
        .centered()
}
