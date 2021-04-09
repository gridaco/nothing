import { Paint } from "canvaskit-wasm";
import { createElement, memo, useMemo } from "react";
import makePaint, { PaintParameters } from "../sk-utils/make-paint";
import makeRect, { RectParameters } from "../sk-utils/make-rect";
import { SKRectComponentProps } from "../types";

interface SKRectProps {
  rect: RectParameters;
  paint: Paint | PaintParameters;
}

export const SKRect = memo(function SKRect(props: SKRectProps) {
  const rect = makeRect(props.rect);
  const paint = makePaint(props.paint);

  const elementProps: SKRectComponentProps = useMemo(
    () => ({
      rect,
      paint,
    }),
    [rect, paint]
  );

  return createElement("SKRect", elementProps);
});
