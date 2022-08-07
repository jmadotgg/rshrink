import init, { run } from "./pkg/rshrink.js";

window.my_func = (nums) => {
  let sum = 0;
  for (let num of nums) {
    sum += num;
  }
  return sum;
};

window.selectedFiles = (selectedFiles) => {
  window.post;
};
window.threadWorker = new Worker("worker.js", { type: "module" });

(async () => {
  await init();
  console.log("Initialized wasm");
  // const threadWorker = new Worker("worker.js", { type: "module" });
  let _app = run("Rshrink");
})();
