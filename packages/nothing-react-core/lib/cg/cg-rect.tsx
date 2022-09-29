import React from "react";
import { Rect as TRect } from "@reflect-ui/uiutils/dist/lib/types";
import { Paint } from "canvaskit-wasm";
import makePaint, {
  PaintParameters,
} from "@nothing-sdk/react-core/lib/sk-utils/make-paint";
import { useCanvaskit } from "../contexts/canvaskit-context";
import { SKRect } from "../sk/rect";
import { color } from "@reflect-ui/core";
import { SKRRect } from "../sk/rrect";

interface SolidFill {
  color: color.RGBAF;
}

/**
 * easier rect usage props
 */
interface CGRectProps {
  x: number;
  y: number;
  width: number;
  background?: SolidFill;
  borderRadius?: number;
  strokeWidth?: number;
  height: number;
  paint?: Paint | PaintParameters;
}

export function CGRect(props: CGRectProps) {
  const { CanvasKit } = useCanvaskit();

  const trect: TRect = {
    x: props.x,
    y: props.y,
    width: props.width,
    height: props.height,
  };

  const fillColor =
    props.background?.color !== undefined
      ? CanvasKit.Color4f(
          props.background.color.r,
          props.background.color.g,
          props.background.color.b,
          props.background.color.a
        )
      : CanvasKit.Color4f(0, 0, 0, 0);

  const paint = new CanvasKit.Paint();
  paint.setStyle(CanvasKit.PaintStyle.Fill);
  paint.setColor(fillColor);

  // const imageFilter = CanvasKit.ImageFilter()
  // paint.setImageFilter()

  const rect = CanvasKit.XYWHRect(trect.x, trect.y, trect.width, trect.height);

  if (props.borderRadius) {
    return (
      <SKRRect borderRadius={props.borderRadius} rect={rect} paint={paint} />
    );
  } else {
    return <SKRect rect={rect} paint={paint} />;
  }
}
