import type { CanvasKit, SkFont, SkPaint } from "canvaskit-oc";
import { isCkCanvas } from "./canvas";
import { toSkFont, toSkPaint } from "../skia-element-mapping";
import {
  CkElement,
  CkElementContainer,
  CkElementCreator,
  CkElementProps,
  CkObjectTyping,
  Paint,
} from "../skia-element-types";

export interface CkRectProps extends CkElementProps<never> {
  paint?: Paint;
  width?: number;
  height?: number;
  x?: number;
  y?: number;
}

class CkRect implements CkElement<"cg-rect"> {
  readonly canvasKit: CanvasKit;
  readonly props: CkObjectTyping["cg-rect"]["props"];
  readonly skObjectType: CkObjectTyping["cg-rect"]["name"] = "Rect";
  readonly type: "cg-rect" = "cg-rect";

  private readonly defaultPaint: SkPaint;

  private renderPaint?: SkPaint;
  deleted = false;

  constructor(canvasKit: CanvasKit, props: CkObjectTyping["cg-rect"]["props"]) {
    this.canvasKit = canvasKit;
    this.props = props;

    this.defaultPaint = new this.canvasKit.SkPaint();
    this.defaultPaint.setStyle(this.canvasKit.PaintStyle.Stroke);
    this.defaultPaint.setAntiAlias(true);
  }

  render(parent?: CkElementContainer<any>): void {
    if (this.deleted) {
      throw new Error("BUG. Rect element deleted.");
    }

    if (parent && isCkCanvas(parent)) {
      // TODO we can be smart and only recreate the paint object if the paint props have changed.
      this.renderPaint?.delete();
      this.renderPaint = toSkPaint(this.canvasKit, this.props.paint);

      parent.skObject?.drawRect(
        {
          fLeft: this.props.x ?? 0,
          fTop: this.props.y ?? 0,
          fRight: this.props.width ?? 100,
          fBottom: this.props.height ?? 100,
        },
        this.defaultPaint
      );
    }
  }

  delete() {
    if (this.deleted) {
      return;
    }
    this.deleted = true;
    this.defaultPaint.delete();
    this.renderPaint?.delete();
  }
}

export const createCkRect: CkElementCreator<"cg-rect"> = (
  type,
  props,
  canvasKit
) => new CkRect(canvasKit, props);
