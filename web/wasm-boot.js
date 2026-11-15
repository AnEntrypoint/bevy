const statusEl = document.getElementById('app-status');
const errorEl = document.getElementById('boot-error');
const statusJson = document.getElementById('status-json');
const showError = (label, error) => {
  const detail = error && (error.stack || error.message || String(error));
  errorEl.style.display = 'block';
  errorEl.textContent = `${label}: ${detail}`;
  statusEl.dataset.webgpu = 'unavailable';
  statusEl.dataset.appReady = 'false';
  statusEl.dataset.blocker = label;
  statusEl.textContent = `WebGPU WASM blocked: ${label}`;
  console.error(`[bevy-wasm] ${label}`, error || '');
};
fetch('./status.json?t=' + Date.now(), { cache: 'no-store' })
  .then((r) => r.ok ? r.json() : Promise.reject(new Error('HTTP ' + r.status)))
  .then((data) => { statusJson.textContent = JSON.stringify(data, null, 2); })
  .catch((e) => { statusJson.textContent = 'status fallback unavailable: ' + e; });
const hasWebGpu = typeof navigator !== 'undefined' && 'gpu' in navigator;
statusEl.dataset.webgpu = hasWebGpu ? 'available' : 'unavailable';
if (!hasWebGpu) {
  statusEl.textContent = 'navigator.gpu unavailable: fallback status shown';
  showError('navigator.gpu unavailable', 'Chromium requires WebGPU; try --enable-unsafe-webgpu --enable-features=Vulkan,UseSkiaRenderer --use-angle=vulkan');
} else {
  statusEl.textContent = 'WebGPU detected; loading Bevy WASM…';
  try {
    const module = await import('./bevy-jayzee-wasm.js');
    await module.default();
    if (typeof module.start !== 'function') throw new Error('wasm-bindgen glue did not export start()');
    module.start();
    console.log('[bevy-wasm] start() returned');
  } catch (error) {
    showError('wasm boot failed', error);
  }
}

window.addEventListener('error', (event) => {
  if (statusEl.dataset.appReady === 'true') return;
  const message = event.message || (event.error && event.error.message) || 'unknown browser error';
  if (/RuntimeError|unreachable|WebGPU|wasm/i.test(message)) {
    showError('headless WebGPU runtime failed', `${message}; use a normal browser with a working WebGPU surface`);
  }
});
window.addEventListener('unhandledrejection', (event) => {
  if (statusEl.dataset.appReady === 'true') return;
  showError('headless WebGPU runtime failed', `${event.reason || 'unknown rejection'}; use a normal browser with a working WebGPU surface`);
});
