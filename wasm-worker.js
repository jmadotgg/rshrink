//import init, { initThreadPool, sum } from "./pkg/rshrink.js";
//console.log("Worker created");
//
//(async () => {
//  await init();
//  console.log("Initialized wasm in worker");
//  await initThreadPool(navigator.hardwareConcurrency);
//  console.log("Initialized thread pool");
//  let summed = sum([1, 2, 3, 4, 5, 6, 8, 0]);
//  console.log("Sum", summed);
//})();
//
//self.onmessage = (msg) => {
//  console.log(msg);
//};

import { threads } from 'wasm-feature-detect';
import * as Comlink from 'comlink';


// Wrap wasm-bindgen exports (Rust functions)
function wrapExports(todo) {
	return todo;
}
async function initHandlers() {
	let [singleThread, multiThread] = await Promise.all([
		(async () => {
			const singleThread = await import('./pkg/rshrink.js');
			await singleThread.default();
			return wrapExports(singleThread);
		})(),
		(async () => {
			// If threads are unsupported int his browser, skipt this handler
			const supportsThreads = await threads();
			if (!supportsThreads) return;
			// TODO: Add feature to support non-threads browsers
			const multiThread = await import(
				'./pkg/rshrink.js'
			)
			await multiThread.default();
			await multiThread.initThreadPool(navigator.hardwareConcurrency);
			return wrapExports(multiThread);
		})()
	]);


	return Comlink.proxy({
		singleThread,
		supportsThreads: !!multiThread,
		multiThread
	});
}

Comlink.expose({
	handlers: initHandlers()
});
