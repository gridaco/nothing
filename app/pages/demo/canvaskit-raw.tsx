import React, { useEffect } from "react";
import CanvasKitInit from "canvaskit-wasm/bin/canvaskit";
import { CanvasKit, Color } from "canvaskit-wasm";

export default function CanvasKitRawDemo() {
  useEffect(() => {
    CanvasKitInit().then((CanvasKit: CanvasKit) => {
      const surface = CanvasKit.MakeCanvasSurface("");
      const canvas = surface.getCanvas();
      const paint = new CanvasKit.Paint();
      const color: Color = new Float32Array([0, 0, 0, 1]);
      paint.setColor(color);
      canvas.drawCircle(0, 0, 50, paint);
    });
  }, []);
  return <div>return skia canvas here</div>;
}
