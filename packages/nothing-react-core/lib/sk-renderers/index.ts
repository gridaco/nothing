import { Canvas, CanvasKit } from "canvaskit-wasm";
import { AnySKElementInstance } from "../types";
import { renderRect } from "./rect";
import { renderPath } from "./path";
import { renderImageRect } from "./image";
import { renderParagraph } from "./paragraph";

export function skrender(
  element: AnySKElementInstance,
  canvas: Canvas,
  canvaskit: CanvasKit
) {
  switch (element.type) {
    case "SKRect": {
      renderRect(element.props, canvas);
      break;
    }
    case "SKPath": {
      renderPath(element.props, canvas);
      break;
    }
    case "SKImage": {
      renderImageRect(element.props, canvas, canvaskit);
      break;
    }
    case "SKText":
      renderParagraph(element.props, canvas);

      break;
    case "Group": {
      const { transform, opacity, clip } = element.props;

      const saveCount = canvas.getSaveCount();

      canvas.save();

      if (clip) {
        if (clip.path instanceof Float32Array) {
          canvas.clipRect(clip.path, clip.op, clip.antiAlias ?? true);
        } else {
          canvas.clipPath(clip.path, clip.op, clip.antiAlias ?? true);
        }
      }

      if (transform) {
        canvas.concat(transform);
      }

      if (opacity < 1) {
        const opacityPaint = new canvaskit.Paint();
        opacityPaint.setAlphaf(opacity);

        canvas.saveLayer(opacityPaint);
      }

      element._elements.forEach((e) => skrender(e, canvas, canvaskit));

      canvas.restoreToCount(saveCount);
      break;
    }
    // default:
    //   throw new Error(`Draw not handled for '${element.type}'`);
  }
}
