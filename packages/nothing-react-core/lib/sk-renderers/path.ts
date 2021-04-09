import { Canvas } from "canvaskit-wasm";
import { SKPathComponentProps } from "../types";

export function renderPath(data: SKPathComponentProps, c: Canvas) {
  c.drawPath(data.path, data.paint);
}
