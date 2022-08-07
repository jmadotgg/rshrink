import init, { run } from "./pkg/rshrink.js";

window.my_func = (nums) => {
  let sum = 0;
  for (let num of nums) {
    sum += num;
  }
  return sum;
};

(async () => {
  await init();
  console.log("Initialized wasm");
  new Worker("worker.js", { type: "module" });
  let _app = run("Rshrink");
})();
