import { BlurStyle, CanvasKit, MaskFilter } from "canvaskit-wasm";

export type { MaskFilter };

export type BlurMaskFilterParameters = {
  style: BlurStyle;
  sigma: number;
  respectCTM: boolean;
};

export function makeBlurMaskFilter(
  ck: CanvasKit,
  params: BlurMaskFilterParameters
): MaskFilter {
  return ck.MaskFilter.MakeBlur(params.style, params.sigma, params.respectCTM);
}
