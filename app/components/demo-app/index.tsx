import { CGRect, Stage } from "@nothing.app/react-core/lib";
import { useState } from "react";

export function DemoInteractiveApp() {
  return <StageContainer />;
}

export function StageContainer() {
  const [size, setSize] = useState(100);
  return (
    <div
      onClick={() => {
        setSize(size + 1);
      }}
    >
      <Stage
        canvasSize={{
          width: 1000,
          height: 500,
        }}
      >
        <CGRect
          x={0}
          y={0}
          width={size}
          height={size}
          background={{
            color: {
              r: 1,
              g: 0,
              b: 1,
              a: 1,
            },
          }}
        />
      </Stage>
    </div>
  );
}
