import { SKImageComponentProps } from "../types";
import { Canvas, CanvasKit } from "canvaskit-wasm";

export function renderImageRect(
  data: SKImageComponentProps,
  c: Canvas,
  ck: CanvasKit
) {
  const { image, rect, paint } = data;
  c.drawImageRectCubic(
    image,
    ck.XYWHRect(0, 0, image.width(), image.height()),
    rect,
    0,
    0,
    paint
  );
}
