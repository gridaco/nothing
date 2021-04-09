import { Canvas, CanvasKit } from "canvaskit-wasm";
import { createContext, useContext } from "react";

export interface CanvasKitContext {
  CanvasKit: CanvasKit;
  canvas: Canvas;
}

const CKContext = createContext<CanvasKitContext | undefined>(undefined);

export const CanvasKitProvider = CKContext.Provider;

export const useCanvaskit = (): CanvasKitContext => {
  const value = useContext(CKContext);

  if (!value) throw new Error(`CanvasKitProvider Not initialized`);

  return value;
};
