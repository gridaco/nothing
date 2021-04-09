import React, { useMemo } from "react";
import MockData from "../../mock/export-node.json";
import { Stage, CGText, CGRect } from "@nothing.app/react-core/lib";

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

function DemoDesignComposition(props: { data: any }) {
  return (
    <>
      {props.data.layers
        .sort((a, b) => a.index - b.index)
        .map((e) => {
          if (e.type == StorableLayerType.text) {
            return (
              <CGText
                x={e.x}
                y={e.y}
                width={e.width}
                text={e.data.text}
                fontSize={e.data.style.fontSize}
                color={e.data.style.color}
              />
            );
          } else if (e.type == StorableLayerType.rect) {
            // e.data.fill ?
            return (
              <CGRect
                x={e.x}
                y={e.y}
                background={{}}
                width={e.width}
                height={e.height}
                // paint={{ color: returnRGBAcolor(e.data.fill) }}
              />
            );
          } else if (e.type == StorableLayerType.vanilla) {
            return (
              <CGRect
                x={e.x}
                y={e.y}
                background={{}}
                width={e.width}
                height={e.height}
              />
            );
          }
        })}
      <CGRect x={10} y={10} width={100} height={100} background={{}} />
    </>
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
      <Stage
        canvasSize={{
          width: canvasWidth,
          height: canvasHeight,
        }}
      >
        <DemoDesignComposition data={MockData.scene} />
      </Stage>
    </div>
  );
}

export default SkiaReflectNode;
