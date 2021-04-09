import React, { useState } from "react";
import type { Surface } from "canvaskit-wasm";
import { render, unmount } from "../..";
import { CSSProperties, memo, useEffect, useLayoutEffect, useRef } from "react";
import styled from "@emotion/styled";
import { loadCanvasKit } from "../../sk/loader";

declare module "canvaskit-wasm" {
  interface Surface {
    flush(): void;
  }
}

const Container = styled.div<{ cursor: CSSProperties["cursor"] }>(
  ({ cursor }) => ({
    flex: "1",
    position: "relative",
    cursor,
  })
);

const Html5CanvasComponent = styled.canvas<{ left: number }>(
  ({ theme, left }) => ({
    position: "absolute",
    top: 0,
    left,
    zIndex: -1,
  })
);

interface StageState {
  // interactionState: InteractionState;
  // highlightedLayer?: LayerHighlight;
  // selectedObjects: string[];
  // selectedSwatchIds: string[];
  // sketch: SketchFile;

  canvasSize: { width: number; height: number };
  canvasInsets: { left: number; right: number };
}

export default memo(function Stage(props: {
  children: React.ReactNode;
  canvasSize: { width: number; height: number };
  canvasInsets: { left: number; right: number };
}) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const CanvasKit = loadCanvasKit();
  const surfaceRef = useRef<Surface | null>(null);
  const [loaded, setLoaded] = useState(false);

  // initially load skia on canvas
  useEffect(() => {
    const canvasElement = canvasRef.current;

    if (!canvasElement) console.log("no canvas ele");

    canvasElement.width = window.innerWidth; //containerSize.width + insets.left + insets.right;
    canvasElement.height = window.innerHeight; //containerSize.height;

    setLoaded(true);

    if (!canvasElement) return;

    if (!surfaceRef.current) {
      const surface = CanvasKit.MakeCanvasSurface(canvasElement);

      if (!surface) {
        surfaceRef.current = null;

        console.error("failed to create surface");
        return;
      }

      surfaceRef.current = surface;
    }

    return () => {
      surfaceRef.current?.delete();
      surfaceRef.current = null;
    };
  }, []);

  // re render via reconciler when initially loaded, or children updated.
  useLayoutEffect(() => {
    if (!surfaceRef.current || surfaceRef.current.isDeleted()) {
      return;
    }

    const surface = surfaceRef.current;
    const context = {
      CanvasKit,
      canvas: surface.getCanvas(),
    };

    try {
      render(props.children, surface, context);

      return () => {
        unmount(surface, context);
      };
    } catch (e) {
      console.error("rendering error", e);
    }
  }, [loaded, props.children]);

  return (
    <Container
      ref={containerRef}
      cursor={"auto"}
      //   onPointerDown={handleMouseDown}
      //   onPointerMove={handleMouseMove}
      //   onPointerUp={handleMouseUp}
    >
      <Html5CanvasComponent ref={canvasRef} left={0} width={0} height={0} />
    </Container>
  );
});
