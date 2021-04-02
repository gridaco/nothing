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
  fLeft: number;
  fTop: number;
  fRight: number;
  fBottom: number;
  /**
   * @description [topLeft, topRight, bottomRight, bottomLeft]
   */
  cornerRadius?: number[];
  rx?: number;
  ry?: number;
}

class CGRRect implements CkElement<"cg-rrect"> {
  readonly canvasKit: CanvasKit;
  readonly props: CkObjectTyping["cg-rrect"]["props"];
  readonly skObjectType: CkObjectTyping["cg-rrect"]["name"] = "RRect";
  readonly type: "cg-rrect" = "cg-rrect";

  private readonly defaultPaint: SkPaint;

  private renderPaint?: SkPaint;
  deleted = false;

  constructor(canvasKit: CanvasKit, props: CkObjectTyping["cg-rect"]["props"]) {
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
      const {
        fLeft,
        fRight,
        fTop,
        fBottom,
        rx,
        ry,
        cornerRadius = [0, 0, 0, 0],
      } = this.props;
      // TODO we can be smart and only recreate the paint object if the paint props have changed.

      this.renderPaint?.delete();
      this.renderPaint = toSkPaint(this.canvasKit, this.props.paint);
      // drawRoundRect also works. It's just coding style choice. - choosing rrect for naming convention solidity.
      if (rx && ry) {
        parent.skObject?.drawRoundRect(
          {
            fLeft,
            fTop,
            fRight,
            fBottom,
          },
          rx ?? 0,
          ry ?? 0,
          this.renderPaint ?? this.defaultPaint
        );
      } else {
        parent.skObject?.drawRRect(
          {
            //SkRect
            rect: {
              fLeft,
              fTop,
              fRight,
              fBottom,
            },
            rx1: cornerRadius[0],
            rx2: cornerRadius[1],
            rx3: cornerRadius[2],
            rx4: cornerRadius[3],
            ry1: cornerRadius[0],
            ry2: cornerRadius[1],
            ry3: cornerRadius[2],
            ry4: cornerRadius[3],
          },
          this.renderPaint ?? this.defaultPaint
        );
      }
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
