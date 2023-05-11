/// <reference types="@webgpu/types" />

interface HTMLCanvasElement extends HTMLElement {
  getContext(contextId: "webgpu"): GPUPresentationContext | null;
}

// Defined by webpack.
declare namespace NodeJS {
  interface Process {
    readonly browser: boolean;
  }

  interface ProcessEnv {
    readonly NODE_ENV: "development" | "production" | "test";
  }
}

declare module "*.wgsl" {
  const shader: string;
  export default shader;
}
