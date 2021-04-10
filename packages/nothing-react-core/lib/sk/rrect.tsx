import { Paint } from "canvaskit-wasm";
import { createElement, memo, useMemo } from "react";
import usePaint, { PaintParameters } from "../sk-utils/make-paint";
import useRRect, { RRectParameters } from "../sk-utils/make-rrect";
import { SKRectComponentProps, SKRRectComponentProps } from "../types";

interface SKRRectProps {
  borderRadius: number;
  rect: RRectParameters;
  paint: Paint | PaintParameters;
}

export const SKRRect = memo(function SKRRect(props: SKRRectProps) {
  const rect = useRRect(props.rect);

  const rrect: Float32Array = Float32Array.from([
    rect[0],
    rect[1],
    rect[2],
    rect[3],
    props.borderRadius, // x
    props.borderRadius, // y
  ]);

  const paint = usePaint(props.paint);

  const elementProps: SKRRectComponentProps = useMemo(
    () => ({
      rrect,
      paint,
    }),
    [rrect, paint]
  );

  return createElement("SKRRect", elementProps);
});
