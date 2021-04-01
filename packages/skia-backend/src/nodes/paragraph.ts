import type {
  CanvasKit,
  SkFontManager,
  SkParagraph,
  SkParagraphStyle,
} from "canvaskit-oc";
import { isCkCanvas } from "./canvas";
import { toSkParagraphStyle } from "../skia-element-mapping";
import {
  CkElement,
  CkElementContainer,
  CkElementCreator,
  CkElementProps,
  CkObjectTyping,
  ParagraphStyle,
} from "../skia-element-types";

export interface CkParagraphProps
  extends ParagraphStyle,
    CkElementProps<SkParagraph> {
  layout: number;
  x?: number;
  y?: number;
  children?: string;
  fontManager?: SkFontManager;
}

class CkParagraph implements CkElement<"cg-paragraph"> {
  readonly canvasKit: CanvasKit;
  readonly props: CkObjectTyping["cg-paragraph"]["props"];
  skObject?: CkObjectTyping["cg-paragraph"]["type"];
  readonly skObjectType: CkObjectTyping["cg-paragraph"]["name"] = "SkParagraph";
  readonly type: "cg-paragraph" = "cg-paragraph";

  deleted = false;

  constructor(
    canvasKit: CanvasKit,
    props: CkObjectTyping["cg-paragraph"]["props"]
  ) {
    this.canvasKit = canvasKit;
    this.props = props;
  }

  render(parent: CkElementContainer<any>): void {
    if (this.deleted) {
      throw new Error("BUG. paragraph element deleted.");
    }

    const skParagraphBuilder = this.canvasKit.ParagraphBuilder.Make(
      <SkParagraphStyle>toSkParagraphStyle(this.canvasKit, this.props),
      this.props.fontManager ?? this.canvasKit.SkFontMgr.RefDefault()
    );
    if (this.props.children) {
      skParagraphBuilder.addText(this.props.children);
    }
    this.skObject?.delete();
    this.skObject = skParagraphBuilder.build();
    this.skObject.layout(this.props.layout);
    if (isCkCanvas(parent)) {
      parent.skObject?.drawParagraph(
        this.skObject,
        this.props.x ?? 0,
        this.props.y ?? 0
      );
    }
    // TODO we can avoid deleting & recreating the paragraph skobject by checkin props that require a new paragraph instance.
  }

  delete() {
    if (this.deleted) {
      return;
    }
    this.deleted = true;
    this.skObject?.delete();
  }
}

export const createCkParagraph: CkElementCreator<"cg-paragraph"> = (
  type,
  props,
  canvasKit
): CkElement<"cg-paragraph"> => new CkParagraph(canvasKit, props);
