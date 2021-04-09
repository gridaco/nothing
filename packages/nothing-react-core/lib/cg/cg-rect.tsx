import React from "react";
import { Rect as TRect } from "@reflect-ui/uiutils/lib/types";
import { Paint } from "canvaskit-wasm";
import makePaint, {
  PaintParameters,
} from "@nothing.app/react-core/lib/sk-utils/make-paint";
import { useCanvaskit } from "../contexts/canvaskit-context";
import { SKRect } from "../sk/rect";

interface SolidFill {
  // color: string;
}

/**
 * easier rect usage props
 */
interface CGRectProps {
  x: number;
  y: number;
  width: number;
  background: SolidFill;
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

  const paint = makePaint({
    style: CanvasKit.PaintStyle.Stroke,
    color: CanvasKit.Color(0, 0, 0, 1),
    strokeWidth: props.strokeWidth,
  });

  const rect = CanvasKit.XYWHRect(trect.x, trect.y, trect.width, trect.height);

  return <SKRect rect={rect} paint={paint} />;
}
