// 1. Some snippets adapted from udevbe/react-canvaskit (MIT License)
// https://github.com/udevbe/react-canvaskit/blob/459c6d804e18b4e6603acc370c961c77244b552f/react-canvaskit/src/ReactCanvasKit.tsx
// 2. this file is a copy of noya-app - https://github.com/noya-app/noya (MIT)
// 3. this file is under nothing.app and being maintained by nothing graphics engine team

import { Surface } from "canvaskit-wasm";
import { fontManager } from "./sk/loader";
import React from "react";
import type { ReactNode } from "react";
import ReactReconciler from "react-reconciler";
import { FontManagerProvider } from "./contexts/font-manager-context";
import { CanvasKitProvider } from "./contexts/canvaskit-context";
import { CanvasKitContext } from "@nothing.app/core/lib";
import { RootComponent } from "@nothing.app/core/lib/types";
import { _hostConfig } from "./hostconfig";

const EXPECTED_REACT_VERSION = "17";
export const __matchRectVersion =
  React.version.split(".")[0] === EXPECTED_REACT_VERSION;

// That warning is useful
if (!__matchRectVersion) {
  const command = `yarn add react@${EXPECTED_REACT_VERSION} react-dom@${EXPECTED_REACT_VERSION}`;
  console.warn(
    `Version mismatch detected for @nothing.app/react-core and react. react-konva expects to have react version ${EXPECTED_REACT_VERSION}, but it has version ${React.version}. Make sure versions are matched, otherwise, @nothing.app/react-core work is not guaranteed. You can use this command: "${command}"`
  );
}

const canvaskitReconciler = ReactReconciler(_hostConfig);

canvaskitReconciler.injectIntoDevTools({
  bundleType: process.env.NODE_ENV !== "production" ? 1 : 0, // 0 for PROD, 1 for DEV
  version: React.version,
  rendererPackageName: "nothing-react-core", // package name
});

function getContainerForSurface(surface: Surface, context: CanvasKitContext) {
  let extendedSurface = surface as Surface & { _container: any };

  if (!extendedSurface._container) {
    const root: RootComponent = { surface, context, children: [] };
    const container = canvaskitReconciler.createContainer(root, 2, false, null);

    extendedSurface._container = container;
  }

  return extendedSurface._container;
}

export function render(
  element: ReactNode,
  surface: Surface,
  context: CanvasKitContext,
  callback?: () => void
) {
  const container = getContainerForSurface(surface, context);

  canvaskitReconciler.updateContainer(
    <CanvasKitProvider value={context}>
      <FontManagerProvider value={fontManager}>{element}</FontManagerProvider>
    </CanvasKitProvider>,
    container,
    null,
    () => {
      callback?.();
    }
  );
}

export function unmount(
  surface: Surface,
  context: CanvasKitContext,
  callback?: () => void
) {
  const container = getContainerForSurface(surface, context);

  canvaskitReconciler.updateContainer(null, container, null, () => {
    callback?.();
  });
}
