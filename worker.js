import init, { initThreadPool } from "./pkg/rshrink.js";
console.log("Worker created");

(async () => {
  await init();
  console.log("Initialized wasm in worker");
  await initThreadPool(navigator.hardwareConcurrency);
  console.log("Initialized thread pool");
})();
