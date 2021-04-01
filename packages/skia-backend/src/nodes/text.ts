import type { CanvasKit, SkFont, SkPaint } from "canvaskit-oc";
import { isCkCanvas } from "./canvas";
import { toSkFont, toSkPaint } from "../skia-element-mapping";
import {
  CkElement,
  CkElementContainer,
  CkElementCreator,
  CkElementProps,
  CkObjectTyping,
  Font,
  Paint,
} from "../skia-element-types";

export interface CkTextProps extends CkElementProps<never> {
  x?: number;
  y?: number;
  paint?: Paint;
  font?: Font;
  children: string;
}

class CkText implements CkElement<"cg-text"> {
  readonly canvasKit: CanvasKit;
  readonly props: CkObjectTyping["cg-text"]["props"];
  readonly skObjectType: CkObjectTyping["cg-text"]["name"] = "Text";
  readonly type: "cg-text" = "cg-text";

  private readonly defaultPaint: SkPaint;
  private readonly defaultFont: SkFont;

  private renderPaint?: SkPaint;
  private renderFont?: SkFont;
  deleted = false;

  constructor(canvasKit: CanvasKit, props: CkObjectTyping["cg-text"]["props"]) {
    this.canvasKit = canvasKit;
    this.props = props;

    this.defaultPaint = new this.canvasKit.SkPaint();
    this.defaultPaint.setStyle(this.canvasKit.PaintStyle.Fill);
    this.defaultPaint.setAntiAlias(true);

    this.defaultFont = new this.canvasKit.SkFont(null, 14);
  }

  render(parent?: CkElementContainer<any>): void {
    if (parent && isCkCanvas(parent)) {
      // TODO we can be smart and only recreate the paint object if the paint props have changed.
      this.renderPaint?.delete();
      this.renderPaint = toSkPaint(this.canvasKit, this.props.paint);
      // TODO we can be smart and only recreate the font object if the font props have changed.
      this.renderFont?.delete();
      this.renderFont = toSkFont(this.canvasKit, this.props.font);
      parent.skObject?.drawText(
        this.props.children,
        this.props.x ?? 0,
        this.props.y ?? 0,
        this.renderPaint ?? this.defaultPaint,
        this.renderFont ?? this.defaultFont
      );
    }
  }

  delete() {
    if (this.deleted) {
      return;
    }
    this.deleted = true;
    this.defaultFont.delete();
    this.defaultPaint.delete();
    this.renderPaint?.delete();
    this.renderFont?.delete();
  }
}

export const createCkText: CkElementCreator<"cg-text"> = (
  type,
  props,
  canvasKit
) => new CkText(canvasKit, props);
