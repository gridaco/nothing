import { Paint, Path } from "canvaskit-wasm";
import { createElement, memo, useMemo } from "react";
import makePaint, { PaintParameters } from "../sk-utils/make-paint";
import { SKPathComponentProps } from "../types";

interface SKPathProps {
  path: Path;
  paint: Paint | PaintParameters;
}

export default memo(function SKPath(props: SKPathProps) {
  const paint = makePaint(props.paint);

  const elementProps: SKPathComponentProps = useMemo(
    () => ({
      paint,
      path: props.path,
    }),
    [paint, props.path]
  );

  return createElement("SKPath", elementProps);
});
