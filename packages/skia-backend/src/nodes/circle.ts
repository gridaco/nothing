import { CanvasKit, SkPaint } from "canvaskit-oc";
import { toSkPaint } from "../skia-element-mapping";
import {
  CkElement,
  CkElementContainer,
  CkElementCreator,
  CkElementProps,
  CkObjectTyping,
  Paint,
} from "../skia-element-types";
import { isCkCanvas } from "./canvas";

export interface CkCircleProps extends CkElementProps<never> {
  paint?: Paint;
  cx: number;
  cy: number;
  radius: number;
}

class CkCircle implements CkElement<"cg-circle"> {
  readonly canvasKit: CanvasKit;
  readonly props: CkCircleProps;
  readonly type: "cg-circle" = "cg-circle";
  readonly skObjectType: CkObjectTyping["cg-circle"]["name"] = "Circle";

  private readonly defaultPaint: SkPaint;

  private renderPaint?: SkPaint;
  deleted = false;

  constructor(
    canvasKit: CanvasKit,
    props: CkObjectTyping["cg-circle"]["props"]
  ) {
    this.canvasKit = canvasKit;
    this.props = props;

    this.defaultPaint = new this.canvasKit.SkPaint();
    this.defaultPaint.setAntiAlias(true);
  }

  render(parent: CkElementContainer<any>): void {
    if (this.deleted) {
      throw new Error("BUG. Circle element deleted.");
    }

    if (parent && isCkCanvas(parent)) {
      const { cx, cy, radius } = this.props;
      this.renderPaint?.delete();
      this.renderPaint = toSkPaint(this.canvasKit, this.props.paint);

      parent.skObject?.drawCircle(
        cx,
        cy,
        radius,
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

export const createCkCircle: CkElementCreator<"cg-circle"> = (
  type,
  props,
  canvasKit
) => new CkCircle(canvasKit, props);
