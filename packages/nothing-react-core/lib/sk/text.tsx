import { Paragraph } from "canvaskit-wasm";
import { createElement, memo, useMemo } from "react";
import { SKTextComponentProps } from "../types";
import makeRect, { RectParameters } from "../sk-utils/make-rect";

interface SKTextProps {
  rect: RectParameters;
  paragraph: Paragraph;
}

export default memo(function SKText(props: SKTextProps) {
  const rect = makeRect(props.rect);
  const elementProps: SKTextComponentProps = useMemo(
    () => ({
      paragraph: props.paragraph,
      rect,
    }),
    [props.paragraph, rect]
  );

  return createElement("SKText", elementProps);
});
