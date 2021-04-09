import type { FontMgr, CanvasKit } from "canvaskit-wasm";
import init from "canvaskit-wasm/bin/canvaskit";

export let fontManager: FontMgr;

/**
 * loads canvas kit wasm, inits font, returns canvaskit instance
 * @returns
 */
export async function load() {
  if (!init) {
    console.error("init failed", init);
    throw "Canvaskit not loaded";
  }
  const [CanvasKit, fontBuffer] = await Promise.all([
    init({
      locateFile: (file: string) =>
        "https://unpkg.com/canvaskit-wasm@0.25.0/bin/" + file,
    }),
    fetch(
      "https://storage.googleapis.com/skia-cdn/google-web-fonts/Roboto-Regular.ttf"
    ).then((resp) => resp.arrayBuffer()),
  ]);

  fontManager = CanvasKit.FontMgr.FromData(fontBuffer)!;

  return CanvasKit;
}

// REGION CANVASKIT LOADER

// initially loads -> this needs to be fixed to be loaded after the manual requets via code access.
// let v: CanvasKit;
// const loadingRequest = load()
//   .then((value: CanvasKit) => {
//     v = value;
//   })
//   .catch((e) => {
//     console.error("failed loading via skia loader", e);
//   });

// export function useCanvasKit() {
//   if (!v) {
//     throw loadingRequest;
//     // throw "canvaskit not loaded. see upper log for failed loading request detail";
//   }
//   return v;
// }

export class SuspendedValue<T> {
  private suspendedPromise: Promise<void>;
  private promiseState: PromiseState<T> = { type: "pending" };

  constructor(promise: Promise<T>) {
    this.suspendedPromise = promise
      .then((value) => {
        this.promiseState = { type: "success", value };
      })
      .catch((value) => {
        this.promiseState = { type: "failure", value };
      });
  }

  getValueOrThrow(): T {
    switch (this.promiseState.type) {
      case "pending":
        console.warn("pending request.");
        throw this.suspendedPromise;
      case "failure":
        console.error("failed to fetch resource");
        throw this.promiseState.value;
      case "success":
        console.log("fetch success");
        return this.promiseState.value;
    }
  }
}

export type PromiseState<T> =
  | {
      type: "pending";
    }
  | {
      type: "success";
      value: T;
    }
  | {
      type: "failure";
      value: Error;
    };

let suspendedCanvasKit = new SuspendedValue<CanvasKit>(load());

export function loadCanvasKit() {
  return suspendedCanvasKit.getValueOrThrow();
}
