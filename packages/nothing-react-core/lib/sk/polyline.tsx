import { CanvasKit, Paint, Path } from "canvaskit-wasm";
import { Point } from "@reflect-ui/uiutils/lib";
import React, { memo, useMemo } from "react";
import { useCanvaskit } from "../contexts/canvaskit-context";
import useDeletable from "../hooks/use-deletable";
import makePaint, { PaintParameters } from "../sk-utils/make-paint";
import SKPath from "./path";

interface SKPolylineProps {
  points: Point[];
  paint: Paint | PaintParameters;
}

function makePath(CanvasKit: CanvasKit, points: Point[]): Path {
  const path = new CanvasKit.Path();

  const [first, ...rest] = points;

  if (!first) return path;

  path.moveTo(first.x, first.y);

  rest.forEach((point) => {
    path.lineTo(point.x, point.y);
  });

  path.close();

  return path;
}

export default memo(function SKPolyline(props: SKPolylineProps) {
  const { CanvasKit } = useCanvaskit();
  const paint = makePaint(props.paint);
  const path = useMemo(() => makePath(CanvasKit, props.points), [
    CanvasKit,
    props.points,
  ]);
  useDeletable(path);

  return <SKPath paint={paint} path={path} />;
});
