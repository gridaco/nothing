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

export interface CGRRectProps extends CkElementProps<never> {
  paint?: Paint;
}

class CGRRect implements CkElement<"cg-rrect"> {
  readonly canvasKit: CanvasKit;
  readonly props: CkObjectTyping["cg-rrect"]["props"];
  readonly skObjectType: CkObjectTyping["cg-rrect"]["name"] = "RRect";
  readonly type: "cg-rrect" = "cg-rrect";

  private readonly defaultPaint: SkPaint;

  private renderPaint?: SkPaint;
  deleted = false;

  constructor(canvasKit: CanvasKit, props: CkObjectTyping["ck-rect"]["props"]) {
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
      // drawRoundRect also works. It's just coding style choice. - choosing rrect for naming convention solidity.
      parent.skObject?.drawRRect(
        {
          //SkRect
          rect: {
            fLeft: 0,
            fTop: 0,
            fRight: 100,
            fBottom: 100,
          },
          rx1: 24,
          rx2: 24,
          rx3: 24,
          rx4: 24,
          ry1: 24,
          ry2: 24,
          ry3: 24,
          ry4: 24,
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

export const createCgRRect: CkElementCreator<"cg-rrect"> = (
  type,
  props,
  canvasKit
) => new CGRRect(canvasKit, props);
