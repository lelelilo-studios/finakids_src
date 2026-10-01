//! Builds character meshes off the main thread: worker threads on native
//! platforms and Web Workers in the browser (see web/worker.js), with a
//! synchronous fallback when neither is available.

use super::appearance::Appearance;
use super::build::{self, CharacterMeshes, FaceLayout, Quality};
use super::skeleton::Skeleton;
use crate::gfx::mesh::{Mat, SkinMeshData, SkinVertex};

/// Skeleton proportions derived from the appearance.
pub fn skeleton_for(app: &Appearance) -> Skeleton {
    let (shoulder, hips) = (1.0 + app.masc * 0.08, 1.0 + (1.0 - app.masc) * 0.05);
    Skeleton::new(app.height, shoulder, hips)
}

pub fn quality_key(q: Quality) -> &'static str {
    match q {
        Quality::High => "high",
        Quality::Low => "low",
        Quality::Mobile => "mobile",
    }
}

/// Builds the meshes for a request key like `sofia:low` or `random:11:high`.
pub fn build_key(key: &str) -> Option<CharacterMeshes> {
    let (app_key, q) = match key.rsplit_once(':') {
        Some((a, "high")) => (a, Quality::High),
        Some((a, "low")) => (a, Quality::Low),
        Some((a, "mobile")) => (a, Quality::Mobile),
        _ => (key, Quality::Low),
    };
    let app = Appearance::by_key(app_key)?;
    let sk = skeleton_for(&app);
    Some(build::build(&app, &sk, q))
}

// ------------------------------------------------------------------ packing

const MAGIC: u32 = 0x464b_4331; // "FKC1"

fn put_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn get_u32(b: &[u8], at: &mut usize) -> Option<u32> {
    let v = u32::from_le_bytes(b.get(*at..*at + 4)?.try_into().ok()?);
    *at += 4;
    Some(v)
}

fn get_f32(b: &[u8], at: &mut usize) -> Option<f32> {
    get_u32(b, at).map(f32::from_bits)
}

/// Serializes meshes so they can cross the Web Worker boundary.
pub fn pack(m: &CharacterMeshes) -> Vec<u8> {
    let verts: &[u8] = bytemuck::cast_slice(&m.skin.verts);
    let idx: &[u8] = bytemuck::cast_slice(&m.skin.indices);
    let face: &[u8] = bytemuck::bytes_of(&m.face);
    let mut out = Vec::with_capacity(verts.len() + idx.len() + face.len() + 64 + m.mats.len() * 20);
    put_u32(&mut out, MAGIC);
    put_u32(&mut out, m.skin.verts.len() as u32);
    put_u32(&mut out, m.skin.indices.len() as u32);
    put_u32(&mut out, face.len() as u32);
    put_u32(&mut out, m.mats.len() as u32);
    out.extend_from_slice(verts);
    out.extend_from_slice(idx);
    out.extend_from_slice(face);
    for mat in &m.mats {
        out.extend_from_slice(&mat.color);
        put_u32(&mut out, mat.rough.to_bits());
        put_u32(&mut out, mat.metal.to_bits());
        put_u32(&mut out, mat.emissive.to_bits());
        out.push(mat.kind);
    }
    out
}

pub fn unpack(b: &[u8]) -> Option<CharacterMeshes> {
    let mut at = 0;
    if get_u32(b, &mut at)? != MAGIC {
        return None;
    }
    let nv = get_u32(b, &mut at)? as usize;
    let ni = get_u32(b, &mut at)? as usize;
    let nf = get_u32(b, &mut at)? as usize;
    let nm = get_u32(b, &mut at)? as usize;
    let vs = std::mem::size_of::<SkinVertex>();
    let vb = b.get(at..at + nv * vs)?;
    at += nv * vs;
    let ib = b.get(at..at + ni * 4)?;
    at += ni * 4;
    let fb = b.get(at..at + nf)?;
    at += nf;
    if nf != std::mem::size_of::<FaceLayout>() {
        return None;
    }
    // the byte buffer has no alignment guarantee: copy element-wise
    let verts: Vec<SkinVertex> = vb.chunks_exact(vs).map(bytemuck::pod_read_unaligned).collect();
    let indices: Vec<u32> = ib.chunks_exact(4).map(|c| u32::from_le_bytes(c.try_into().unwrap())).collect();
    let face: FaceLayout = bytemuck::pod_read_unaligned(fb);
    let mut mats = Vec::with_capacity(nm);
    for _ in 0..nm {
        let color: [u8; 4] = b.get(at..at + 4)?.try_into().ok()?;
        at += 4;
        let rough = get_f32(b, &mut at)?;
        let metal = get_f32(b, &mut at)?;
        let emissive = get_f32(b, &mut at)?;
        let kind = *b.get(at)?;
        at += 1;
        mats.push(Mat {
            color,
            rough,
            metal,
            emissive,
            kind,
        });
    }
    Some(CharacterMeshes {
        skin: SkinMeshData { verts, indices },
        face,
        mats,
    })
}

/// Entry point for the Web Worker (web/worker.js).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn build_character_packed(key: &str) -> Vec<u8> {
    console_error_panic_hook::set_once();
    match build_key(key) {
        Some(m) => pack(&m),
        None => Vec::new(),
    }
}

// ------------------------------------------------------------------ factory

#[cfg(target_arch = "wasm32")]
mod js {
    use wasm_bindgen::prelude::*;
    #[wasm_bindgen]
    extern "C" {
        /// Queues a build on the page's worker pool; false if workers are unavailable.
        #[wasm_bindgen(js_name = __fkBuild, catch)]
        pub fn fk_build(id: u32, key: &str) -> Result<bool, JsValue>;
        /// Next finished build as `{id, bytes}` (or `{id, error}`), or null.
        #[wasm_bindgen(js_name = __fkPoll, catch)]
        pub fn fk_poll() -> Result<JsValue, JsValue>;
    }
}

struct Job {
    id: u32,
    key: String,
}

pub struct CharFactory {
    next_id: u32,
    /// Jobs waiting for the synchronous fallback, one per `poll`.
    local: Vec<Job>,
    in_flight: usize,
    #[cfg(not(target_arch = "wasm32"))]
    tx: Option<std::sync::mpsc::Sender<Job>>,
    #[cfg(not(target_arch = "wasm32"))]
    rx: std::sync::mpsc::Receiver<(u32, Option<CharacterMeshes>)>,
    #[cfg(target_arch = "wasm32")]
    web_keys: Vec<(u32, String)>,
}

impl CharFactory {
    pub fn new() -> CharFactory {
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::sync::{mpsc, Arc, Mutex};
            let (tx, jobs) = mpsc::channel::<Job>();
            let (done_tx, rx) = mpsc::channel();
            let jobs = Arc::new(Mutex::new(jobs));
            let n = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(2).clamp(2, 5) - 1;
            let mut ok = false;
            for i in 0..n {
                let jobs = jobs.clone();
                let done = done_tx.clone();
                let spawned = std::thread::Builder::new()
                    .name(format!("char-build-{i}"))
                    .stack_size(8 << 20)
                    .spawn(move || loop {
                        let job = {
                            let Ok(guard) = jobs.lock() else { return };
                            match guard.recv() {
                                Ok(j) => j,
                                Err(_) => return,
                            }
                        };
                        let m = build_key(&job.key);
                        if done.send((job.id, m)).is_err() {
                            return;
                        }
                    });
                ok |= spawned.is_ok();
            }
            CharFactory {
                next_id: 1,
                local: Vec::new(),
                in_flight: 0,
                tx: if ok { Some(tx) } else { None },
                rx,
            }
        }
        #[cfg(target_arch = "wasm32")]
        CharFactory {
            next_id: 1,
            local: Vec::new(),
            in_flight: 0,
            web_keys: Vec::new(),
        }
    }

    /// Queues a build and returns its ticket.
    pub fn request(&mut self, key: &str) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.in_flight += 1;
        let job = Job { id, key: key.to_string() };
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(tx) = &self.tx {
                if let Err(e) = tx.send(job) {
                    self.local.push(e.0);
                }
            } else {
                self.local.push(job);
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            match js::fk_build(id, key) {
                Ok(true) => self.web_keys.push((id, job.key)),
                _ => self.local.push(job),
            }
        }
        id
    }

    pub fn pending(&self) -> usize {
        self.in_flight
    }

    /// Returns one finished build if available. With the synchronous fallback
    /// this builds one character per call.
    pub fn poll(&mut self) -> Option<(u32, CharacterMeshes)> {
        #[cfg(not(target_arch = "wasm32"))]
        if let Ok((id, m)) = self.rx.try_recv() {
            self.in_flight -= 1;
            return Some((id, m.expect("appearance key")));
        }
        #[cfg(target_arch = "wasm32")]
        if let Ok(v) = js::fk_poll() {
            if !v.is_null() && !v.is_undefined() {
                use wasm_bindgen::JsCast;
                let id = js_sys::Reflect::get(&v, &"id".into()).ok().and_then(|x| x.as_f64()).unwrap_or(0.0) as u32;
                let bytes = js_sys::Reflect::get(&v, &"bytes".into())
                    .ok()
                    .and_then(|b| b.dyn_into::<js_sys::Uint8Array>().ok())
                    .map(|a| a.to_vec());
                if let Some(pos) = self.web_keys.iter().position(|k| k.0 == id) {
                    let (_, key) = self.web_keys.remove(pos);
                    match bytes.as_deref().and_then(unpack) {
                        Some(m) => {
                            self.in_flight -= 1;
                            return Some((id, m));
                        }
                        None => {
                            log::warn!("worker build failed for {key}; building on the main thread");
                            self.local.push(Job { id, key });
                        }
                    }
                }
            }
        }
        if !self.local.is_empty() {
            let job = self.local.remove(0);
            self.in_flight -= 1;
            return Some((job.id, build_key(&job.key).expect("appearance key")));
        }
        None
    }

    /// Blocks until a build finishes (native only; used by debug starts and screenshots).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn wait(&mut self) -> Option<(u32, CharacterMeshes)> {
        if self.in_flight == 0 {
            return None;
        }
        if let Some(r) = self.poll() {
            return Some(r);
        }
        let (id, m) = self.rx.recv().ok()?;
        self.in_flight -= 1;
        Some((id, m.expect("appearance key")))
    }
}

impl Default for CharFactory {
    fn default() -> Self {
        Self::new()
    }
}
