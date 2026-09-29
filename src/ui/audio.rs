use std::cell::RefCell;
use std::num::NonZero;
use std::sync::Arc;
use std::time::Duration;

use gpui_kit::component::ActiveTheme;
use gpui_kit::component::button::Button;
use gpui_kit::{Context, IntoElement, ParentElement, Render, Styled, Window, div, px, relative};
use rodio::mixer::Mixer;
use rodio::{ChannelCount, DeviceSinkBuilder, MixerDeviceSink, Player, SampleRate, nz};

use crate::outputs::Audio;
use crate::ui::style::{CONTROL_GAP, card};

thread_local! {
    // Note that the `MixerDeviceSink` must be kept alive as long as any
    // `Player` built from its mixer is in use. It isn't `Send` (it owns a
    // cpal stream), so share it via thread-local storage instead.
    static DEVICE_SINK: RefCell<Option<MixerDeviceSink>> = RefCell::new(None);
}

/// Runs `f` with the thread's shared output device's mixer, opening the
/// default output device on first use.
fn with_mixer<R>(f: impl FnOnce(&Mixer) -> R) -> R {
    DEVICE_SINK.with_borrow_mut(|device_sink| {
        let device_sink =
            device_sink.get_or_insert_with(|| DeviceSinkBuilder::open_default_sink().unwrap());
        f(device_sink.mixer())
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
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    #[inline]
    fn channels(&self) -> ChannelCount {
        nz!(1)
    }
    #[inline]
    fn sample_rate(&self) -> SampleRate {
        NonZero::new(self.audio.sr).expect("sample rate must be non-zero")
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

pub struct AudioPlayer {
    audio: Audio,
    // `Player`'s own methods (`play`/`pause`/`append`/...) all take `&self`,
    // so an `Arc` is enough to share it with the polling task spawned below
    // — no `Mutex` needed on top.
    player: Arc<Player>,
    playing: bool,
    progress: f32,
}

impl AudioPlayer {
    pub fn new(audio: Audio) -> Self {
        let player = with_mixer(Player::connect_new);

        AudioPlayer {
            audio,
            player: Arc::new(player),
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
        let player = self.player.clone();
        let total = self.audio.length_in_sec();
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(16))
                    .await;
                let (playing, elapsed_fraction) = if player.empty() {
                    (false, 0.0)
                } else {
                    // A paused sink isn't `empty()`, so it must be checked
                    // separately — otherwise `playing` flips back to `true`
                    // within one tick of pausing (`toggle`'s own `pause()`
                    // call already set it to `false`), and the button label
                    // never leaves "Pause". `get_pos()` still reflects the
                    // frozen position while paused, so the bar doesn't jump.
                    let paused = player.is_paused();
                    let pos = player.get_pos().as_secs_f32();
                    (!paused, (pos / total.max(1e-6)).clamp(0.0, 1.0))
                };
                let should_stop = this
                    .update(cx, |this, cx| {
                        this.playing = playing;
                        // `elapsed_fraction` is 0.0 once the sink is empty, i.e.
                        // when playback has ended: the bar resets at once (see
                        // `Render`, which draws it without any animation).
                        this.progress = elapsed_fraction;
                        cx.notify();
                        !playing
                    })
                    .unwrap_or(true);
                if should_stop {
                    break;
                }
            }
        })
        .detach();
    }

    fn toggle(&mut self, cx: &mut Context<Self>) {
        let just_started_or_resumed = if self.player.empty() {
            self.player.append(AudioWrapper {
                audio: self.audio.clone(),
                num_sample: 0,
            });
            self.player.play();
            self.playing = true;
            true
        } else if self.playing {
            self.player.pause();
            self.playing = false;
            false
        } else {
            self.player.play();
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
        // A plain track + fill instead of gpui-kit's `Progress`: that
        // component animates every value change, so the bar visibly ran
        // *backwards* when playback ended and the value reset to 0 (and
        // lagged behind the 16ms position updates while playing).
        card(cx)
            .flex()
            .flex_row()
            .gap(CONTROL_GAP)
            .items_center()
            .child(
                // Fixed width: "Play" and "Pause" differ in width, which
                // would otherwise resize the whole card on every toggle.
                div().w(px(72.)).child(
                    Button::new("audio-toggle")
                        .label(label)
                        .w_full()
                        .on_click(cx.listener(|this, _, _window, cx| this.toggle(cx))),
                ),
            )
            .child(
                div()
                    .w(px(200.))
                    .h(px(6.))
                    .rounded_full()
                    .bg(cx.theme().progress_bar.opacity(0.25))
                    .child(
                        div()
                            .h_full()
                            .w(relative(self.progress.clamp(0.0, 1.0)))
                            .rounded_full()
                            .bg(cx.theme().progress_bar),
                    ),
            )
    }
}
