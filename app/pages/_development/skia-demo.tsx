import type { SkParagraph } from "canvaskit-oc";
import React, { useEffect } from "react";
import {
  init,
  render,
  PaintStyle,
  TextAlignEnum,
  useFontManager,
} from "@nothing.app/skia-backend";
import useAnimationFrame from "../../components/use-animation-frame";
import { Stage } from "@nothing.app/skia-backend/src/stage";
function Comp() {
  return (
    <ck-canvas clear={{ red: 255, green: 165, blue: 0 }}>
      <ck-text
        x={5}
        y={50}
        paint={{ color: "#00FFFF", antiAlias: true }}
        font={{ size: 24 }}
      >
        Nothing engine
      </ck-text>
      <ck-rect />
      <ck-surface width={100} height={100} dx={100} dy={100}>
        <ck-canvas clear="#FF00FF" rotate={{ degree: 45 }}>
          <ck-text>Nothing engine with skia backend, running on react</ck-text>
          <ck-line
            x1={0}
            y1={10}
            x2={142}
            y2={10}
            paint={{ antiAlias: true, color: "#FFFFFF", strokeWidth: 10 }}
          />
        </ck-canvas>
      </ck-surface>
    </ck-canvas>
  );
}

export default function Page() {
  return (
    <Stage width={400} height={400}>
      <Comp />
    </Stage>
  );
}

// export default function Skia3rdPartyDemoPage() {
//   useEffect(() => {
//     if (process.browser) {
//       const htmlCanvasElement = document.createElement("canvas");
//       const rootElement = document.getElementById("root");
//       rootElement.appendChild(htmlCanvasElement);
//       document.body.appendChild(htmlCanvasElement);
//       htmlCanvasElement.width = 400;
//       htmlCanvasElement.height = 300;
//       init().then(() => render(<Comp />, htmlCanvasElement));
//     }
//   }, []);
//   return <div id="root"></div>;
// }
