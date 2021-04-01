import React, { useEffect } from "react";
import { ReactNodeList } from "react-reconciler";
import { init, render } from "./canvaskit";

const SK_CANVAS_ID = "__skia_backend_canvas";
export function Stage(props: {
  width: number;
  height: number;
  children: ReactNodeList;
}) {
  useEffect(() => {
    const ready =
      // nextjs
      ("browser" in process && (process as any).browser) ||
      // general browser
      document !== undefined;
    if (ready) {
      const htmlCanvas: HTMLCanvasElement = document.getElementById(
        SK_CANVAS_ID
      ) as HTMLCanvasElement;
      init().then(() => render(props.children, htmlCanvas));
    }
  }, []);
  return (
    <>
      <canvas id={SK_CANVAS_ID} width={props.width} height={props.height} />
    </>
  );
}
