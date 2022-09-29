import { useMemo } from "react";
import { useCanvaskit } from "../contexts/canvaskit-context";
import useDeletable from "../hooks/use-deletable";
import {
  makeBlurMaskFilter as _makeBlurMaskFilter,
  BlurMaskFilterParameters,
  MaskFilter,
} from "@nothing-sdk/core/lib/sk-utils/make-blur-mask-filter";

export default function makeBlurMaskFilter(
  parameters: BlurMaskFilterParameters
): MaskFilter {
  const { CanvasKit } = useCanvaskit();

  const maskFilter = useMemo(
    () => _makeBlurMaskFilter(CanvasKit, parameters),
    [CanvasKit.MaskFilter, Object.values(parameters)]
  );

  return useDeletable(maskFilter);
}
