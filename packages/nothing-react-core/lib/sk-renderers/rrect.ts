import { SKRRectComponentProps } from "../types";
import { Canvas, CanvasKit } from "canvaskit-wasm";

export function renderRRect(data: SKRRectComponentProps, c: Canvas) {
  const { rrect: rect, paint } = data;
  c.drawRRect(rect, paint);
}
