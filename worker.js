import init, { initThreadPool, sum } from "./pkg/rshrink.js";
console.log("Worker created");

(async () => {
  await init();
  console.log("Initialized wasm in worker");
  await initThreadPool(navigator.hardwareConcurrency);
  console.log("Initialized thread pool");
  let summed = sum([1, 2, 3, 4, 5, 6, 8, 0]);
  console.log("Sum", summed);
})();

self.onmessage = (msg) => {
  console.log(msg);
};
