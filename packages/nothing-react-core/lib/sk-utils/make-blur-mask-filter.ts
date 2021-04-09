import { BlurStyle, MaskFilter } from "canvaskit-wasm";
import { useMemo } from "react";
import { useCanvaskit } from "../contexts/canvaskit-context";
import useDeletable from "../hooks/use-deletable";

export type BlurMaskFilterParameters = {
  style: BlurStyle;
  sigma: number;
  respectCTM: boolean;
};

export default function makeBlurMaskFilter(
  parameters: BlurMaskFilterParameters
): MaskFilter {
  const { CanvasKit } = useCanvaskit();

  const maskFilter = useMemo(
    () =>
      CanvasKit.MaskFilter.MakeBlur(
        parameters.style,
        parameters.sigma,
        parameters.respectCTM
      ),
    [
      CanvasKit.MaskFilter,
      parameters.respectCTM,
      parameters.sigma,
      parameters.style,
    ]
  );

  return useDeletable(maskFilter);
}
