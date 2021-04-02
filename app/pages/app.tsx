import React from "react";
import { Stage } from "@nothing.app/skia-backend/src/stage";
import MockData from "../mock/export-node.json";

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

function returnRGBAcolor(fill) {
  let { r, g, b, a: alpha } = fill;

  const hex =
    ((alpha * 255) | (1 << 8)).toString(16).slice(1) +
    ((r * 255) | (1 << 8)).toString(16).slice(1) +
    ((g * 255) | (1 << 8)).toString(16).slice(1) +
    ((b * 255) | (1 << 8)).toString(16).slice(1);

  var _r = parseInt(hex.slice(1, 3), 16),
    _g = parseInt(hex.slice(3, 5), 16),
    _b = parseInt(hex.slice(5, 7), 16);

  let a = 1;

  try {
    if (hex.length >= 8) {
      // 8 or 9 if '#' included in hex, then 9, if not, 8
      a = parseInt(hex.slice(7, 9), 16);
    }
  } catch (_) {}

  return {
    red: _r,
    green: _g,
    blue: _b,
    alpha: a,
  };
}

function CanvasComposition(props: { data: any }) {
  return (
    <cg-canvas>
      {/* <cg-rect width={props.data.width} height={props.data.height} /> */}
      {props.data.layers
        .sort((a, b) => a.index - b.index)
        .map((e) => {
          if (e.type == StorableLayerType.text) {
            /**
             * @description line break is not working on the skia engine, I've taken care of it like this.
             */
            return e.data.text.split("\n").map((i, ix) => {
              return (
                <cg-text
                  x={e.x}
                  y={e.y + ix * e.data.style.fontSize}
                  font={{ size: e.data.style.fontSize }}
                  paint={{ color: { red: 255, green: 0, blue: 0 } }}
                >
                  {i}
                </cg-text>
              );
            });
          } else if (e.type == StorableLayerType.vanilla) {
            return (
              <cg-surface width={e.width} height={e.height} dx={e.x} dy={e.y}>
                <cg-canvas clear="#999" />
              </cg-surface>
            );
          } else if (e.type == StorableLayerType.rect) {
            return e.data.fill ? (
              <cg-rect
                fBottom={e.width + e.y}
                fTop={e.y}
                fLeft={e.x}
                fRight={e.height + e.x}
                paint={{ color: returnRGBAcolor(e.data.fill) }}
              />
            ) : (
              <cg-rect
                fBottom={e.width + e.y}
                fTop={e.y}
                fLeft={e.x}
                fRight={e.height + e.x}
              />
            );

            // p1 = x, y

            //   return (
            //     <cg-rect
            //       width={e.width}
            //       height={e.height}
            //     />
            //   );
            // } else {
            //   console.log(e);
            //   return (
            //     <cg-rect x={e.x} y={e.y} width={e.width} height={e.height} />
            //   );
            // }
          }
        })}
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
