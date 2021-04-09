import { Paint } from "canvaskit-wasm";
import { useMemo } from "react";
import { useCanvaskit } from "../contexts/canvaskit-context";
import { ColorParameters } from "./make-color";
import makePaint, { PaintParameters } from "./make-paint";

export function makeFill(parameters: Omit<PaintParameters, "style">): Paint {
  const { CanvasKit } = useCanvaskit();

  const parametersWithStyle = useMemo(
    () => ({
      ...parameters,
      style: CanvasKit.PaintStyle.Fill,
    }),
    [CanvasKit.PaintStyle.Fill, parameters]
  );

  return makePaint(parametersWithStyle);
}

export function useColorFill(color: ColorParameters): Paint {
  const { CanvasKit } = useCanvaskit();

  const parametersWithStyle = useMemo(
    () => ({
      color,
      style: CanvasKit.PaintStyle.Fill,
    }),
    [CanvasKit.PaintStyle.Fill, color]
  );

  return makePaint(parametersWithStyle);
}
