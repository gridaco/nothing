import { color } from "@reflect-ui/core";
import { SKText, useCanvaskit, fontManager } from "@nothing-sdk/react-core/lib";
import { Rect as TRect } from "@reflect-ui/uiutils/dist/lib/types";

// TODO make CGText, CGTextSpan, CGRichText - reference flutter's
// This currently not supported mixed text styles
export function CGText(props: {
  x: number;
  y: number;
  text: string;
  width: number;
  height?: number;
  fontSize?: number;
  color?: color.RGBAF;
  fontFamily?: string;
}) {
  const { CanvasKit } = useCanvaskit();

  const paragraph = () => {
    const paragraphStyle = new CanvasKit.ParagraphStyle({
      textStyle: {
        color: CanvasKit.BLACK,
        fontFamilies: ["Roboto"],
      },
      textAlign: CanvasKit.TextAlign.Left,
      // maxLines: .., // TODO
      ellipsis: "...",
    });

    const builder = CanvasKit.ParagraphBuilder.Make(
      paragraphStyle,
      fontManager
    );

    // layer.attributedString.attributes.forEach((attribute) => {
    //   const { location, length } = attribute;
    //   const string = layer.attributedString.string.substr(location, length);
    //   const style = Primitives.stringAttribute(CanvasKit, attribute);
    //   builder.pushStyle(style);
    //   builder.addText(string);
    //   builder.pop();
    // });

    const style = new CanvasKit.TextStyle({
      fontSize: props.fontSize ?? 12,
      fontFamilies: [props.fontFamily ?? "Roboto"],
      fontStyle: {
        weight: CanvasKit.FontWeight.Black,
      },
      color: CanvasKit.Color4f(
        props.color.r,
        props.color.g,
        props.color.b,
        props.color.a
      ),
    });

    builder.pushStyle(style);
    builder.addText(props.text);
    builder.pop();

    /**
       * backgroundColor?: InputColor;
      color?: InputColor;
      decoration?: number;
      decorationColor?: InputColor;
      decorationThickness?: number;
      decrationStyle?: DecorationStyle;
      fontFamilies?: string[];
      fontFeatures?: TextFontFeatures[];
      fontSize?: number;
      fontStyle?: FontStyle;
      foregroundColor?: InputColor;
      heightMultiplier?: number;
      letterSpacing?: number;
      locale?: string;
      shadows?: TextShadow[];
      textBaseline?: TextBaseline;
      wordSpacing?: number;
       */

    const paragraph = builder.build();
    paragraph.layout(props.width);

    return paragraph;
  };

  const trect: TRect = {
    x: props.x,
    y: props.y,
    width: props.width,
    height: props.height,
  };

  const rect = CanvasKit.XYWHRect(trect.x, trect.y, trect.width, trect.height);

  return <SKText rect={rect} paragraph={paragraph()} />;
}
