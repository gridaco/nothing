import { ImageResponse } from "next/og";

export const alt = "Nothing — the Grida graphics engine";
export const size = {
  width: 1200,
  height: 630,
};
export const contentType = "image/png";

export default function Image() {
  return new ImageResponse(
    <div
      style={{
        width: "100%",
        height: "100%",
        display: "flex",
        flexDirection: "column",
        justifyContent: "space-between",
        color: "#0a0a0a",
        background: "#ffffff",
        padding: "64px 72px",
        fontFamily: "Arial, Helvetica, sans-serif",
      }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 20,
          fontSize: 34,
          fontWeight: 700,
          letterSpacing: "-0.04em",
        }}
      >
        <div
          style={{
            width: 44,
            height: 44,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            border: "3px solid #0a0a0a",
            borderRadius: 8,
            color: "#0a0a0a",
            fontSize: 24,
          }}
        >
          n0
        </div>
        nothing
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 24 }}>
        <div
          style={{
            maxWidth: 940,
            fontSize: 78,
            lineHeight: 0.98,
            fontWeight: 700,
            letterSpacing: "-0.065em",
          }}
        >
          A graphics engine with nothing in the way.
        </div>
        <div style={{ color: "#666666", fontSize: 28 }}>
          Rust · WebAssembly · WebGL · the .grida format
        </div>
      </div>
    </div>,
    size
  );
}
