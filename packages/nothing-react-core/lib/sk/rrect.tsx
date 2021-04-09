import { Paint } from "canvaskit-wasm";
import { createElement, memo, useMemo } from "react";
import usePaint, { PaintParameters } from "../sk-utils/make-paint";
import useRRect, { RRectParameters } from "../sk-utils/make-rrect";
import { SKRectComponentProps } from "../types";

interface SKRRectProps {
  rect: RRectParameters;
  paint: Paint | PaintParameters;
}

export default memo(function SKRRect(props: SKRRectProps) {
  const rect = useRRect(props.rect);
  const paint = usePaint(props.paint);

  const elementProps: SKRectComponentProps = useMemo(
    () => ({
      rect,
      paint,
    }),
    [rect, paint]
  );

  return createElement("SKRRect", elementProps);
});
