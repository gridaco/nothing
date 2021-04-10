import React, { useEffect, useMemo, useState } from "react";
import MockData from "../../mock/export-node.json";
import {
  Stage,
  CGText,
  CGRect,
  SKImage,
  useCanvaskit,
} from "@nothing.app/react-core/lib";
import useImage from "@nothing.app/use-image";
import makePaint from "@nothing.app/react-core/lib/sk-utils/make-paint";

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
            return (
              <CGRect
                x={e.x}
                y={e.y}
                borderRadius={e.data.borderRadius?.all}
                background={{
                  color: e.data.fill,
                }}
                width={e.width}
                height={e.height}
                // paint={{ color: returnRGBAcolor(e.data.fill) }}
              />
            );
          } else if (e.type == StorableLayerType.vanilla) {
            return (
              <CGImage x={e.x} y={e.y} width={e.width} height={e.height} />
            );
          }
        })}
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

const DUMMY_IMAGE =
  "https://d1csarkz8obe9u.cloudfront.net/posterpreviews/artistic-album-cover-design-template-d12ef0296af80b58363dc0deef077ecc_screen.jpg?ts=1561488440";
const DUMMY_IMAGE_CORS = `https://cors.bridged.cc/${DUMMY_IMAGE}`;
function CGImage(props: {
  x: number;
  y: number;
  width: number;
  height: number;
}) {
  const { CanvasKit: ck } = useCanvaskit();

  // const [image] = useImage(DUMMY_IMAGE);
  const [image, setImage] = useState<ArrayBuffer>();
  useEffect(() => {
    fetch(DUMMY_IMAGE_CORS).then((r) => {
      r.arrayBuffer().then((ab) => {
        setImage(ab);
      });
    });
  }, []);

  const rect = ck.XYWHRect(props.x, props.y, props.width, props.height);

  // const paint = makePaint({colo})
  const paint = new ck.Paint();

  return <SKImage image={image} rect={rect} paint={paint} />;
}
