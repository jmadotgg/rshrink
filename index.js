//
//window.my_func = (nums) => {
//  let sum = 0;
//  for (let num of nums) {
//    sum += num;
//  }
//  return sum;
//};
//
//window.selectedFiles = (selectedFiles) => {
//  window.post;
//};
//window.threadWorker = new Worker("worker.js", { type: "module" });
//
//(async () => {
//  await init();
//  console.log("Initialized wasm");
//  // const threadWorker = new Worker("worker.js", { type: "module" });
//  let _app = run("Rshrink");
//})();

import * as Comlink from 'comlink';
import init, { run } from "./pkg/rshrink.js";
let handlers;


console.myFunc = () => {
	console.log("my_func")
};

//export function myFunc {
//	console.log("my_func");
//}


// Init
(async () => {
// Create a separate thread from wasm-worker.js and get a proxy to its handlers
	await init();
	handlers = await Comlink.wrap(
		new  Worker(new URL("./wasm-worker.js", import.meta.url), {
			type: 'module'
		})
	).handlers;

	console.log("Waiting for thread support");
	console.log(handlers.singleThread)
	if (await handlers.supportsThreads) {
		console.log("Supports threads");
	} else {
		console.log("Does not support threads");
	}

	let _app = run("Rshrink");
})()
