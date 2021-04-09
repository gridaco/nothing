import { Paint, PaintStyle, MaskFilter } from "canvaskit-wasm";
import { useMemo } from "react";
import { useCanvaskit } from "../contexts/canvaskit-context";
import makeColor, { ColorParameters } from "./make-color";
import useDeletable from "../hooks/use-deletable";

export interface PaintParameters {
  color: ColorParameters;
  opacity?: number;
  style: PaintStyle;
  strokeWidth?: number;
  antiAlias?: boolean;
  maskFilter?: MaskFilter;
}

function isPaint(value: Paint | PaintParameters): value is Paint {
  return "delete" in value;
}

export default function makePaint(parameters: PaintParameters | Paint): Paint {
  const { CanvasKit } = useCanvaskit();

  const maybePaintObject = isPaint(parameters) ? parameters : undefined;
  const maybeParameters = makeStablePaintParameters(
    !isPaint(parameters) ? parameters : undefined
  );

  const paint = useMemo(() => {
    if (!maybeParameters) return;

    const paint = new CanvasKit.Paint();
    paint.setColor(maybeParameters.color);
    paint.setStyle(maybeParameters.style);
    paint.setAntiAlias(maybeParameters.antiAlias ?? true);
    paint.setStrokeWidth(maybeParameters.strokeWidth ?? 1);

    if (maybeParameters.opacity !== undefined && maybeParameters.opacity < 1) {
      paint.setAlphaf(maybeParameters.opacity);
    }

    if (maybeParameters.maskFilter) {
      paint.setMaskFilter(maybeParameters.maskFilter);
    }

    return paint;
  }, [maybeParameters, CanvasKit.Paint]);

  const deletablePaint = useDeletable(paint);

  return maybePaintObject ?? deletablePaint!;
}

function makeStablePaintParameters(parameters: PaintParameters | undefined) {
  const maybeColor = makeColor(parameters?.color);

  const paint = useMemo(
    () => {
      if (!maybeColor || !parameters) return;

      return {
        color: maybeColor!,
        style: parameters.style,
        antiAlias: parameters.antiAlias,
        strokeWidth: parameters.strokeWidth,
        maskFilter: parameters.maskFilter,
        opacity: parameters.opacity,
      };
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [
      maybeColor,
      parameters?.style,
      parameters?.antiAlias,
      parameters?.strokeWidth,
      parameters?.maskFilter,
      parameters?.opacity,
    ]
  );

  return paint;
}
