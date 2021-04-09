import { memoize } from "@reflect-ui/uiutils/lib";
import { CanvasKit, Paint, Image } from "canvaskit-wasm";
import { createElement, memo, useMemo } from "react";
import makePaint, { PaintParameters } from "../sk-utils/make-paint";
import makeRect, { RectParameters } from "../sk-utils/make-rect";
import { SKImageComponentProps } from "../types";
import { useCanvaskit } from "../contexts/canvaskit-context";

const decodeImage = memoize(
  (
    CanvasKit: CanvasKit,
    data: ArrayBuffer
  ): ReturnType<CanvasKit["MakeImageFromEncoded"]> => {
    return CanvasKit.MakeImageFromEncoded(data);
  }
);

interface SKImageProps {
  image: Image | ArrayBuffer;
  rect: RectParameters;
  paint: Paint | PaintParameters;
}

export default memo(function SKImage(props: SKImageProps) {
  const { CanvasKit } = useCanvaskit();

  const rect = makeRect(props.rect);
  const paint = makePaint(props.paint);
  const image = useMemo(
    () =>
      props.image instanceof ArrayBuffer
        ? decodeImage(CanvasKit, props.image)
        : props.image,
    [CanvasKit, props.image]
  );

  const elementProps: SKImageComponentProps | undefined = useMemo(
    () =>
      image
        ? {
            rect,
            paint,
            image,
          }
        : undefined,
    [rect, paint, image]
  );

  if (!elementProps) return null;

  return createElement("SKImage", elementProps);
});
