import type { CanvasKit, SkCanvas } from "canvaskit-oc";
import type { ReactNode } from "react";
import { isCkSurface } from "./surface";
import { toSkColor } from "../skia-element-mapping";
import {
  CkElement,
  CkElementContainer,
  CkElementCreator,
  CkElementProps,
  CkObjectTyping,
  Color,
} from "../skia-element-types";

export interface CkCanvasProps extends CkElementProps<SkCanvas> {
  clear?: Color | string;
  rotate?: { degree: number; px?: number; py?: number };
  children?: ReactNode;
}

type CkCanvasChild = CkElement<"cg-surface"> | CkElement<"cg-text">;

export class CkCanvas implements CkElementContainer<"cg-canvas"> {
  readonly canvasKit: CanvasKit;
  readonly props: CkObjectTyping["cg-canvas"]["props"];
  skObject?: CkObjectTyping["cg-canvas"]["type"];
  readonly skObjectType: CkObjectTyping["cg-canvas"]["name"] = "SkCanvas";
  readonly type: "cg-canvas" = "cg-canvas";
  children: CkCanvasChild[] = [];

  private deleted = false;

  constructor(
    canvasKit: CanvasKit,
    props: CkObjectTyping["cg-canvas"]["props"]
  ) {
    this.canvasKit = canvasKit;
    this.props = props;
  }

  render(parent: CkElementContainer<any>): void {
    if (this.deleted) {
      throw new Error("BUG. canvas element deleted.");
    }

    if (parent.skObject && isCkSurface(parent)) {
      if (this.skObject === undefined) {
        this.skObject = parent.skObject.getCanvas();
      }
    } else {
      throw new Error(
        "Expected an initialized SKSurface as parent of cg-canvas"
      );
    }

    this.skObject.save();
    this.drawSelf(this.skObject);
    this.children.forEach((child) => child.render(this));
    this.skObject.restore();
    this.skObject.flush();
  }

  private drawSelf(skCanvas: SkCanvas) {
    const skColor = toSkColor(this.canvasKit, this.props.clear);
    if (skColor) {
      skCanvas.clear(skColor);
    }

    if (this.props.rotate) {
      const { degree, px, py } = this.props.rotate;
      skCanvas.rotate(degree, px ?? 0, py ?? 0);
    }
  }

  delete() {
    if (this.deleted) {
      return;
    }
    this.deleted = true;
    // The canvas object is 1-to-1 linked to the parent surface object, so deleting it means we could never recreate it.
    // this.skObject?.delete()
    this.skObject = undefined;
  }
}

export function isCkCanvas(ckElement: CkElement<any>): ckElement is CkCanvas {
  return ckElement.type === "cg-canvas";
}

export const createCkCanvas: CkElementCreator<"cg-canvas"> = (
  type,
  props,
  canvasKit: CanvasKit
): CkElementContainer<"cg-canvas"> => new CkCanvas(canvasKit, props);
