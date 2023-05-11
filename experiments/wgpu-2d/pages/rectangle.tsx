import React, { useEffect } from "react";

const vertexShader = `
  @vertex
  fn main(@location(0) pos: vec2<f32>) -> @builtin(position) vec4<f32> {
    return vec4<f32>(pos, 0.0, 1.0);
  }
`;

const fragmentShader = `
  struct Uniforms {
    color: vec4<f32>
  };
  
  @binding(0) @group(0) var<uniform> uniforms: Uniforms;

  @fragment
  fn main() -> @location(0) vec4<f32> {
    return uniforms.color;
  }
`;

const swapChainFormat = "bgra8unorm";

async function init(canvas: HTMLCanvasElement) {
  const adapter = await navigator.gpu.requestAdapter();
  const device = await adapter!.requestDevice();
  const context = canvas.getContext("webgpu") as GPUCanvasContext;

  context.configure({
    device,
    format: swapChainFormat,
  });

  async function createPipeline(
    device: GPUDevice,
    vertexShader: string,
    fragmentShader: string
  ): Promise<GPURenderPipeline> {
    const pipeline = device.createRenderPipeline({
      vertex: {
        module: device.createShaderModule({
          code: vertexShader,
        }),
        entryPoint: "main",
        buffers: [
          {
            arrayStride: 8,
            attributes: [
              {
                shaderLocation: 0,
                offset: 0,
                format: "float32x2" as any,
              },
            ],
          },
        ],
      },
      fragment: {
        module: device.createShaderModule({
          code: fragmentShader,
        }),
        entryPoint: "main",
        targets: [
          {
            format: swapChainFormat as any,
          },
        ],
      },
      primitive: {
        topology: "triangle-list",
      },
      layout: "auto",
    });

    return pipeline;
  }

  async function drawRectangle(
    width: number,
    height: number,
    color: [number, number, number, number]
  ): Promise<void> {
    const pipeline = await createPipeline(device, vertexShader, fragmentShader);

    const vertexData = new Float32Array([
      -width / 2,
      -height / 2,
      width / 2,
      -height / 2,
      width / 2,
      height / 2,
      -width / 2,
      height / 2,
    ]);

    const vertexBuffer = device.createBuffer({
      size: vertexData.byteLength,
      usage: GPUBufferUsage.VERTEX,
      mappedAtCreation: true,
    });
    new Float32Array(vertexBuffer.getMappedRange()).set(vertexData);
    vertexBuffer.unmap();

    const uniformBuffer = device.createBuffer({
      size: 4 * 4,
      usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
    });

    const bindGroupLayout = pipeline.getBindGroupLayout(0);
    const bindGroup = device.createBindGroup({
      layout: bindGroupLayout,
      entries: [
        {
          binding: 0,
          resource: {
            buffer: uniformBuffer,
          },
        },
      ],
    });

    const commandEncoder = device.createCommandEncoder();
    const textureView = context.getCurrentTexture().createView();

    const renderPassDescriptor: GPURenderPassDescriptor = {
      colorAttachments: [
        {
          view: textureView,
          clearValue: { r: 0, g: 0, b: 0, a: 1 },
          loadOp: "clear" as any,
          storeOp: "store" as any,
        },
      ],
    };

    const renderPass = commandEncoder.beginRenderPass(renderPassDescriptor);
    renderPass.setPipeline(pipeline);
    renderPass.setVertexBuffer(0, vertexBuffer);
    renderPass.setBindGroup(0, bindGroup);
    renderPass.draw(4, 1, 0, 0);
    renderPass.end();

    const colorBuffer = new Float32Array(color);
    device.queue.writeBuffer(
      uniformBuffer,
      0,
      colorBuffer.buffer,
      colorBuffer.byteOffset,
      colorBuffer.byteLength
    );

    device.queue.submit([commandEncoder.finish()]);
  }

  drawRectangle(1, 1, [1, 1, 1, 1]);
}

export default function Rectangle() {
  const ref = React.useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = ref.current;
    if (canvas === null) return;

    init(canvas);

    // const ctx = canvas.getContext("2d");

    // if (ctx === null) return;

    // ctx.fillStyle = "red";
    // ctx.fillRect(0, 0, 100, 100);
  }, []);

  return (
    <>
      <canvas ref={ref} width={500} height={500} />
    </>
  );
}
