import React, { useEffect } from "react";
import { Stage } from "@nothing.app/skia-backend/src/stage";
function Comp() {
  return (
    <cg-canvas clear={{ red: 255, green: 165, blue: 0 }}>
      <cg-text
        x={5}
        y={50}
        paint={{ color: "#00FFFF", antiAlias: true }}
        font={{ size: 24 }}
      >
        Nothing engine
      </cg-text>
      {/* <cg-rect /> */}
      {/* <cg-rrect /> */}
      <cg-image />
      <cg-surface width={100} height={100} dx={100} dy={100}>
        <cg-canvas clear="#FF00FF" rotate={{ degree: 45 }}>
          <cg-text>Nothing engine with skia backend, running on react</cg-text>
          <cg-line
            x1={0}
            y1={10}
            x2={142}
            y2={10}
            paint={{ antiAlias: true, color: "#FFFFFF", strokeWidth: 10 }}
          />
        </cg-canvas>
      </cg-surface>
    </cg-canvas>
  );
}

export default function Page() {
  return (
    <Stage width={400} height={400}>
      <Comp />
    </Stage>
  );
}
