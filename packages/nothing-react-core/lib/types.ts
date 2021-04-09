import { ReactNode } from "react";
import { CanvasKitContext } from "./contexts/canvaskit-context";
import {
  ClipOp,
  Image,
  Matrix3x3,
  Paint,
  Paragraph,
  Path,
  Rect,
  RRect,
  Surface,
} from "canvaskit-wasm";

// region rect
export interface SKRectComponentProps {
  rect: Rect;
  paint: Paint;
}

interface SKRectComponent {
  type: "SKRect";
  props: SKRectComponentProps;
}
// endregion rect

// region rrect
export interface SKRRectComponentProps {
  rect: RRect;
  paint: Paint;
}

interface SKRRectComponent {
  type: "SKRRect";
  props: SKRRectComponentProps;
}
// endregion rrect

// region image
export interface SKImageComponentProps {
  rect: Rect;
  image: Image;
  paint: Paint;
}

interface SKImageComponent {
  type: "SKImage";
  props: SKImageComponentProps;
}
// endregion image

// region path
export interface SKPathComponentProps {
  path: Path;
  paint: Paint;
}

interface SKPathComponent {
  type: "SKPath";
  props: SKPathComponentProps;
}
// endregion path

// region text
export interface SKTextComponentProps {
  rect: Rect;
  paragraph: Paragraph;
}

interface SKTextComponent {
  type: "SKText";
  props: SKTextComponentProps;
}
// endregion text

// region clip
export interface ClipProps {
  path: Float32Array | Path;
  op: ClipOp;
  antiAlias?: boolean;
}
// endregion clip

// region group
export interface GroupComponentProps {
  transform?: Matrix3x3;
  opacity: number;
  children: ReactNode;
  clip?: ClipProps;
}

interface GroupComponent {
  type: "Group";
  props: GroupComponentProps;
  _elements: AnySKElementInstance[];
}
// endregion group

export interface SKElementTypeMap {
  // SK Core components
  SKRect: SKRectComponent;
  SKRRect: SKRRectComponent;
  SKText: SKTextComponent;
  SKPath: SKPathComponent;
  SKImage: SKImageComponent;
  //
  Group: GroupComponent;
}

export type SKElementType = keyof SKElementTypeMap;
export type SKElementInstance<K extends SKElementType> = SKElementTypeMap[K];
export type SKElementProps<
  K extends SKElementType
> = SKElementInstance<K>["props"];

export type AnySKElementInstance = SKElementInstance<SKElementType>;
export type AnySKElementProps = SKElementProps<SKElementType>;

export interface RootComponent {
  context: CanvasKitContext;
  surface: Surface;
  children: SKElementInstance<SKElementType>[];
}
