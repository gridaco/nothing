import { Color } from "canvaskit-wasm";
import { useMemo } from "react";
import { useCanvaskit } from "../contexts/canvaskit-context";
import makeStable4ElementArray from "./make-stable-4element-array";

export type ColorParameters = Color | number[] | string;

/**
 * makes given color input as skia compat color value
 * @param parameters
 */
export default function makeColor(parameters: ColorParameters): Color;
export default function makeColor(
  parameters: ColorParameters | undefined
): Color | undefined;
export default function makeColor(
  parameters: ColorParameters | undefined
): Color | undefined {
  const { CanvasKit } = useCanvaskit();

  const color = useMemo(() => {
    if (parameters instanceof Float32Array) return parameters;
    if (parameters instanceof Array) return new Float32Array(parameters);
    if (parameters === undefined) return parameters;
    return CanvasKit.parseColorString(parameters);
  }, [CanvasKit, parameters]);

  return makeStable4ElementArray(color as Float32Array);
}
