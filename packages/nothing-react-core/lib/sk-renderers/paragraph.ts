import { Canvas } from "canvaskit-wasm";
import { SKTextComponentProps } from "../types";

export function renderParagraph(d: SKTextComponentProps, c: Canvas) {
  const { rect, paragraph } = d;
  c.drawParagraph(paragraph, rect[0], rect[1]);
}
