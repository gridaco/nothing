import { SKRectComponentProps } from "../types";
import { Canvas, CanvasKit } from "canvaskit-wasm";

export function renderRect(data: SKRectComponentProps, c: Canvas) {
  const { rect, paint } = data;
  c.drawRect(rect, paint);
}
