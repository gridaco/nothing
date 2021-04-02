import React from "react";
import useImage from "use-image";
import { Stage } from "@nothing.app/skia-backend/src/stage";
import MockData from "../../mock/export-node.json";

const canvasWidth = 500;
const canvasHeight = 1000;

enum StorableLayerType {
  instance = "INSTANCE",
  group = "GROUP",
  vanilla = "VANILLA",
  text = "TEXT",
  line = "LINE",
  vector = "VECTOR",
  image = "IMAGE",
  rect = "RECT",
}

function CanvasComposition(props: { data: any }) {
  return (
    <cg-canvas>
      <cg-surface
        width={props.data.width}
        height={props.data.height}
        dx={(canvasWidth - props.data.width) / 2}
        dy={(canvasHeight - props.data.height) / 2}
      >
        <cg-canvas>
          {props.data.layers
            .sort((a, b) => a.index - b.index)
            .map((e) => {
              if (e.type == StorableLayerType.text) {
                return (
                  <cg-text x={e.x} y={e.y}>
                    {e.data.text}
                  </cg-text>
                );
              } else if (e.type == StorableLayerType.vanilla) {
                // return (
                //   <cg-surface
                //     width={e.width}
                //     height={e.height}
                //     dx={e.x}
                //     dy={e.y}
                //   >
                //     <cg-canvas clear="#000" />
                //   </cg-surface>
                // );
              } else if (e.type == StorableLayerType.rect) {
                // return <cg-rect paint={{ color: "red" }} />;
              }
            })}
        </cg-canvas>
      </cg-surface>
    </cg-canvas>
  );
}

function SkiaReflectNode() {
  return (
    <div
      style={{
        width: "100vw",
        height: "100vh",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
      }}
    >
      <Stage width={canvasWidth} height={canvasHeight}>
        <CanvasComposition data={MockData.scene} />
      </Stage>
    </div>
  );
}

export default SkiaReflectNode;
