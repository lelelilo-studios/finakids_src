// Builds character meshes off the main thread. Loads the same wasm module as
// the page (without starting the game) and returns packed mesh bytes.
import init, { build_character_packed } from './pkg/finakids.js';

let ready = null;

self.onmessage = async (e) => {
  const d = e.data;
  if ('init' in d) {
    ready = init(d.init ? { module_or_path: d.init } : undefined)
      .then(() => self.postMessage({ ready: true }))
      .catch((err) => self.postMessage({ fatal: String(err) }));
    return;
  }
  await ready;
  const t0 = performance.now();
  const bytes = build_character_packed(d.key);
  self.postMessage({ id: d.id, bytes, ms: Math.round(performance.now() - t0) }, [bytes.buffer]);
};
