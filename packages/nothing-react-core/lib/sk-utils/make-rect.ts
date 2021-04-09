import { Rect } from "canvaskit-wasm";
import makeStable4ElementArray from "./make-stable-4element-array";

export type RectParameters = Float32Array;

export default function makeRect(parameters: RectParameters): Rect {
  return makeStable4ElementArray(parameters);
}
