import dynamic from "next/dynamic";
import React, { useEffect } from "react";
import CanvasKitInit from "canvaskit-wasm/bin/canvaskit";
import type { CanvasKit, Color } from "canvaskit-wasm";

export default function CanvasKitRawDemo() {
  let color0 = {
    value: "#4746cd",
  };
  useEffect(() => {
    CanvasKitInit({
      locateFile: (file) =>
        "https://unpkg.com/canvaskit-wasm@0.25.0/bin/" + file,
    }).then((CanvasKit: CanvasKit) => {
      // One can specify up to 10 sliders or color pickers using the syntax
      // #sliderN:displayNameNoSpaces. This will create a variable in the scope
      // matching the left part (it's a normal HTML input tag) that has the part
      // on the right as the display name in the html. #colorN is also valid for
      // a native HTML color picker.
      // #slider0:strokeWidth #color0:dashColor

      const surface = CanvasKit.MakeCanvasSurface("canvas");
      if (!surface) {
        console.log("Could not make surface");
        return;
      }
      const skcanvas = surface.getCanvas();
      const paint = new CanvasKit.Paint();

      let offset = 0;
      let X = 250;
      let Y = 250;

      // If there are multiple contexts on the screen, we need to make sure
      // we switch to this one before we draw.
      const context = CanvasKit.currentContext();

      function getColor() {
        // color0.value is in #RRGGBB form
        // https://developer.mozilla.org/en-US/docs/Web/HTML/Element/input/color
        return CanvasKit.parseColorString(color0.value);
      }

      function getWidth() {
        // slider0.valueAsNumber is a float in the range [0, 1]
        return 0.5 * 10 + 3;
      }

      function drawFrame() {
        // Including a call to benchmarkFPS() [a provided helper function]
        // will spawn an HTML widget that is
        // updated every 10 frames with the average framerate over those last
        // 10 frames.
        // benchmarkFPS();
        const path = starPath(CanvasKit, X, Y);
        CanvasKit.setCurrentContext(context);
        const dpe = CanvasKit.PathEffect.MakeDash([15, 5, 5, 10], offset / 5);
        offset++;

        paint.setPathEffect(dpe);
        paint.setStyle(CanvasKit.PaintStyle.Stroke);
        paint.setStrokeWidth(getWidth());
        paint.setAntiAlias(true);
        paint.setColor(getColor());

        skcanvas.clear(CanvasKit.Color(255, 255, 255, 1.0));

        skcanvas.drawPath(path, paint);
        //@ts-ignore
        skcanvas.flush();
        dpe.delete();
        path.delete();

        // Animation loops can continue running even when clicking Run again.
        // It is recommended to call isRunning() [a provided helper function]
        // before requesting another animation frame.
        // if (isRunning()) {
        requestAnimationFrame(drawFrame);
        // }
      }
      requestAnimationFrame(drawFrame);

      function starPath(CanvasKit, X, Y, R = 128) {
        let p = new CanvasKit.Path();
        p.moveTo(X + R, Y);
        for (let i = 1; i < 8; i++) {
          let a = 2.6927937 * i;
          p.lineTo(X + R * Math.cos(a), Y + R * Math.sin(a));
        }
        return p;
      }

      // Make animation interactive
      // canvas.addEventListener("mousemove", (e) => {
      //   X = e.offsetX;
      //   Y = e.offsetY;
      // });

      // const surface = c.MakeCanvasSurface("canvas");

      // const canvas = surface.getCanvas();
      // console.log("canvas", canvas);
      // const paint = new c.Paint();
      // const color: Color = c.Color(100, 100, 100, 1);
      // paint.setColor(color);
      // canvas.drawText("hi", 0, 0, paint, new c.Font());
      // const circ = canvas.drawCircle(10, 10, 50, paint);
      // console.log("circ", circ);
    });
  }, []);
  return (
    <div
      style={{
        width: 500,
        height: 500,
      }}
    >
      <canvas
        style={{
          width: "100%",
          height: "100%",
        }}
        id="canvas"
      />
    </div>
  );
}
