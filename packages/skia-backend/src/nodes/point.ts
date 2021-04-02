import { CanvasKit, SkPaint, SkPoint, SkPointMode } from "canvaskit-oc";
import { isCkCanvas } from ".";
import { toSkPaint } from "../skia-element-mapping";
import {
  CkElement,
  CkElementContainer,
  CkElementCreator,
  CkElementProps,
  CkObjectTyping,
  Paint,
} from "../skia-element-types";

export interface CkPointProps extends CkElementProps<never> {
  paint?: Paint;
  mode: number;
  points: SkPoint[];
}

class CkPoint implements CkElement<"cg-point"> {
  readonly canvasKit: CanvasKit;
  readonly props: CkPointProps;
  readonly type: "cg-point" = "cg-point";
  readonly skObjectType: CkObjectTyping["cg-point"]["name"] = "Point";

  private readonly defaultPaint: SkPaint;

  private renderPaint?: SkPaint;
  deleted = false;

  constructor(
    canvasKit: CanvasKit,
    props: CkObjectTyping["cg-point"]["props"]
  ) {
    this.canvasKit = canvasKit;
    this.props = props;

    this.defaultPaint = new this.canvasKit.SkPaint();
    this.defaultPaint.setStyle(this.canvasKit.PaintStyle.Fill);
    this.defaultPaint.setAntiAlias(true);
  }

  render(parent: CkElementContainer<any>): void {
    if (this.deleted) {
      throw new Error("BUG. Circle element deleted.");
    }

    if (parent && isCkCanvas(parent)) {
      const { mode, points } = this.props;
      this.renderPaint?.delete();
      this.renderPaint = toSkPaint(this.canvasKit, this.props.paint);

      parent.skObject?.drawPoints(
        (mode as unknown) as SkPointMode,
        points,
        this.renderPaint ?? this.defaultPaint
      );
    }
  }

  delete(): void {
    if (this.deleted) {
      return;
    }
    this.deleted = true;
    this.defaultPaint.delete();
    this.renderPaint?.delete();
  }
}

export const createCkPoint: CkElementCreator<"cg-point"> = (
  type,
  props,
  canvasKit
) => new CkPoint(canvasKit, props);
