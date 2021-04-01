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
  x?: number;
  y?: number;
  paint?: Paint;
  children: string;
}

class CkRect implements CkElement<"ck-text"> {
  readonly canvasKit: CanvasKit;
  readonly props: CkObjectTyping["ck-text"]["props"];
  readonly skObjectType: CkObjectTyping["ck-text"]["name"] = "Text";
  readonly type: "ck-text" = "ck-text";

  private readonly defaultPaint: SkPaint;

  private renderPaint?: SkPaint;
  deleted = false;

  constructor(canvasKit: CanvasKit, props: CkObjectTyping["ck-text"]["props"]) {
    this.canvasKit = canvasKit;
    this.props = props;

    this.defaultPaint = new this.canvasKit.SkPaint();
    this.defaultPaint.setStyle(this.canvasKit.PaintStyle.Fill);
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
          fLeft: 0,
          fTop: 0,
          fRight: 100,
          fBottom: 100,
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

export const createCkRect: CkElementCreator<"ck-text"> = (
  type,
  props,
  canvasKit
) => new CkRect(canvasKit, props);
