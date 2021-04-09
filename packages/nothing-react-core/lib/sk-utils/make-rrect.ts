import { RRect } from "canvaskit-wasm";
import useStable4ElementArray from "./make-stable-4element-array";

export type RRectParameters = Float32Array;

export default function useRRect(parameters: RRectParameters): RRect {
  return useStable4ElementArray(parameters);
}
