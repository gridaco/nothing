import { createContext, useContext } from "react";
import { CanvasKitContext } from "@nothing-sdk/core/lib";

const CKContext = createContext<CanvasKitContext | undefined>(undefined);

export const CanvasKitProvider = CKContext.Provider;

export const useCanvaskit = (): CanvasKitContext => {
  const value = useContext(CKContext);

  if (!value) throw new Error(`CanvasKitProvider Not initialized`);

  return value;
};
