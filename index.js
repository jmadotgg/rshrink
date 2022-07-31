import init, { run } from "./pkg/rshrink.js";

(async () => {
  await init();
  console.log("Initialized wasm");
  new Worker("worker.js", { type: "module" });
  let _app = run("Rshrink");
})();
