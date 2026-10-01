//! Native output through cpal (ALSA, WASAPI, CoreAudio, AAudio).
//!
//! A generator thread renders the mix a few tens of milliseconds ahead into a
//! lock-free ring; the real-time callback only copies samples out of it, so it
//! never allocates, locks or synthesizes.

use super::{Cmd, Mixer};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;

/// Single-producer single-consumer ring of f32 samples.
struct Ring {
    buf: Box<[UnsafeCell<f32>]>,
    mask: usize,
    head: AtomicUsize,
    tail: AtomicUsize,
}

// SAFETY: exactly one thread pushes (generator) and one pops (audio callback);
// each only writes its own index and the slots between tail and head are never
// touched by both at the same time.
unsafe impl Send for Ring {}
unsafe impl Sync for Ring {}

impl Ring {
    fn new(cap_pow2: usize) -> Ring {
        Ring {
            buf: (0..cap_pow2).map(|_| UnsafeCell::new(0.0)).collect(),
            mask: cap_pow2 - 1,
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }
    fn len(&self) -> usize {
        self.head.load(Ordering::Acquire).wrapping_sub(self.tail.load(Ordering::Acquire))
    }
    fn push(&self, data: &[f32]) -> usize {
        let head = self.head.load(Ordering::Relaxed);
        let free = self.buf.len() - head.wrapping_sub(self.tail.load(Ordering::Acquire));
        let n = data.len().min(free);
        for (i, s) in data[..n].iter().enumerate() {
            unsafe { *self.buf[(head.wrapping_add(i)) & self.mask].get() = *s };
        }
        self.head.store(head.wrapping_add(n), Ordering::Release);
        n
    }
    fn pop(&self, out: &mut [f32]) -> usize {
        let tail = self.tail.load(Ordering::Relaxed);
        let avail = self.head.load(Ordering::Acquire).wrapping_sub(tail);
        let n = out.len().min(avail);
        for (i, s) in out[..n].iter_mut().enumerate() {
            *s = unsafe { *self.buf[(tail.wrapping_add(i)) & self.mask].get() };
        }
        self.tail.store(tail.wrapping_add(n), Ordering::Release);
        n
    }
}

struct Shared {
    ring: Ring,
    alive: AtomicBool,
    /// Largest block (frames) the device has asked for.
    need: AtomicUsize,
    /// Debug counters: callbacks served and callbacks that ran dry.
    callbacks: AtomicUsize,
    underruns: AtomicUsize,
    /// Set once the generator has produced its first block.
    started: AtomicBool,
}

pub struct Native {
    tx: Sender<Cmd>,
    shared: Arc<Shared>,
    stream: cpal::Stream,
    paused: bool,
}

impl Drop for Native {
    fn drop(&mut self) {
        self.shared.alive.store(false, Ordering::Relaxed);
    }
}

fn build<T>(device: &cpal::Device, config: cpal::StreamConfig, shared: Arc<Shared>) -> Option<cpal::Stream>
where
    T: cpal::SizedSample + cpal::FromSample<f32>,
{
    let ch = config.channels as usize;
    let mut scratch = vec![0.0f32; 4096];
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                let frames = data.len() / ch.max(1);
                if frames > shared.need.load(Ordering::Relaxed) {
                    shared.need.store(frames, Ordering::Relaxed);
                }
                let mut done = 0;
                while done < frames {
                    let n = (frames - done).min(scratch.len() / 2);
                    let got = shared.ring.pop(&mut scratch[..n * 2]);
                    // underrun: play silence rather than garbage
                    scratch[got..n * 2].fill(0.0);
                    shared.callbacks.fetch_add(1, Ordering::Relaxed);
                    if got < n * 2 && shared.started.load(Ordering::Relaxed) {
                        shared.underruns.fetch_add(1, Ordering::Relaxed);
                    }
                    for f in 0..n {
                        let (l, r) = (scratch[f * 2], scratch[f * 2 + 1]);
                        let o = &mut data[(done + f) * ch..(done + f + 1) * ch];
                        if ch == 1 {
                            o[0] = T::from_sample((l + r) * 0.5);
                        } else {
                            o[0] = T::from_sample(l);
                            o[1] = T::from_sample(r);
                            for x in &mut o[2..] {
                                *x = T::from_sample(0.0f32);
                            }
                        }
                    }
                    done += n;
                }
            },
            {
                // a lost device can report the same error on every callback: log a few only
                let mut shown = 0u32;
                move |e| {
                    if shown < 4 {
                        shown += 1;
                        log::warn!("audio: {e}");
                    }
                }
            },
            None,
        )
        .ok()
}

impl Native {
    pub fn new() -> Option<Native> {
        let host = cpal::default_host();
        let device = host.default_output_device()?;
        let supported = device.default_output_config().ok()?;
        let format = supported.sample_format();
        let config: cpal::StreamConfig = supported.config();
        let sr = config.sample_rate as f32;
        if !(8_000.0..=192_000.0).contains(&sr) || config.channels == 0 {
            return None;
        }
        let shared = Arc::new(Shared {
            ring: Ring::new(1 << 16),
            alive: AtomicBool::new(true),
            need: AtomicUsize::new(0),
            callbacks: AtomicUsize::new(0),
            underruns: AtomicUsize::new(0),
            started: AtomicBool::new(false),
        });
        let (tx, rx) = channel::<Cmd>();
        let sh = shared.clone();
        std::thread::Builder::new()
            .name("finakids-audio".into())
            .spawn(move || {
                let mut mix = Mixer::new(sr);
                mix.prewarm();
                let mut block = vec![0.0f32; 512];
                let mut paused = false;
                // extra safety margin, grown whenever the device runs dry
                let mut margin = 0usize;
                let mut seen_underruns = 0usize;
                while sh.alive.load(Ordering::Relaxed) {
                    while let Ok(c) = rx.try_recv() {
                        if let Cmd::Paused(p) = c {
                            paused = p;
                            // re-prime the buffer after a pause before counting underruns
                            sh.started.store(false, Ordering::Relaxed);
                        }
                        mix.handle(c);
                    }
                    if paused {
                        std::thread::sleep(std::time::Duration::from_millis(30));
                        continue;
                    }
                    // stay ~70 ms ahead (or two device blocks if those are larger);
                    // on a busy machine that keeps starving us, buffer more
                    let under = sh.underruns.load(Ordering::Relaxed);
                    if under > seen_underruns {
                        seen_underruns = under;
                        margin = (margin + (sr * 0.03) as usize).min((sr * 0.18) as usize);
                    }
                    let need = sh.need.load(Ordering::Relaxed);
                    let target = (((sr * 0.07) as usize).max(need * 2 + 256) + margin).min(28_000);
                    if sh.ring.len() / 2 < target {
                        mix.render(&mut block);
                        sh.ring.push(&block);
                    } else {
                        // the buffer is primed: from now on a dry callback is a real underrun
                        sh.started.store(true, Ordering::Relaxed);
                        std::thread::sleep(std::time::Duration::from_millis(2));
                    }
                }
            })
            .ok()?;
        use cpal::SampleFormat as F;
        let stream = match format {
            F::F32 => build::<f32>(&device, config, shared.clone()),
            F::I16 => build::<i16>(&device, config, shared.clone()),
            F::U16 => build::<u16>(&device, config, shared.clone()),
            F::I32 => build::<i32>(&device, config, shared.clone()),
            F::U8 => build::<u8>(&device, config, shared.clone()),
            F::I8 => build::<i8>(&device, config, shared.clone()),
            F::F64 => build::<f64>(&device, config, shared.clone()),
            _ => None,
        };
        let Some(stream) = stream else {
            shared.alive.store(false, Ordering::Relaxed);
            return None;
        };
        if stream.play().is_err() {
            shared.alive.store(false, Ordering::Relaxed);
            return None;
        }
        log::info!("audio: {} Hz, {} canales, {:?}", sr, config.channels, format);
        Some(Native {
            tx,
            shared,
            stream,
            paused: false,
        })
    }

    pub fn send(&mut self, c: Cmd) {
        if let Cmd::Paused(p) = c {
            if p != self.paused {
                self.paused = p;
                let _ = if p { self.stream.pause() } else { self.stream.play() };
            }
        }
        let _ = self.tx.send(c);
    }

    pub fn update(&mut self) {}

    /// (callbacks, underruns, largest device block in frames, frames buffered).
    pub fn stats(&self) -> (usize, usize, usize, usize) {
        (
            self.shared.callbacks.load(Ordering::Relaxed),
            self.shared.underruns.load(Ordering::Relaxed),
            self.shared.need.load(Ordering::Relaxed),
            self.shared.ring.len() / 2,
        )
    }
}
