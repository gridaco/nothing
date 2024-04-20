struct VertexInput {
  @location(0) position: vec2f,
  @builtin(instance_index) instance: u32
};

struct VertexOutput {
  @builtin(position) position: vec4f,
  @location(1) @interpolate(flat) instance: u32,
  @location(2) @interpolate(linear) vertex: vec2f,
};

struct Rectangle {
  color: vec4f,
  position: vec2f,
  _unused: f32,
  sigma: f32,
  corners: vec4f,
  size: vec2f,
  window: vec2f,
};

struct UniformStorage {
  rectangles: array<Rectangle>,
};

@group(0) @binding(0) var<storage> data: UniformStorage;

// To be honest this is a huge overkill. I tried to find what is the least
// correct value that still works without changing how things look and
// funnily enough it's 3. Not 3.14, just 3. But let's keep it for the sake
// of it.
const pi = 3.141592653589793;

// Adapted from https://madebyevan.com/shaders/fast-rounded-rectangle-shadows/
fn gaussian(x: f32, sigma: f32) -> f32 {
  return exp(-(x * x) / (2 * sigma * sigma)) / (sqrt(2 * pi) * sigma);
}

// This approximates the error function, needed for the gaussian integral.
fn erf(x: vec2f) -> vec2f {
  let s = sign(x);
  let a = abs(x);
  var result = 1 + (0.278393 + (0.230389 + 0.078108 * (a * a)) * a) * a;
  result = result * result;
  return s - s / (result * result);
}

fn selectCorner(x: f32, y: f32, c: vec4f) -> f32 {
  return mix(mix(c.x, c.y, step(0, x)), mix(c.w, c.z, step(0, x)), step(0, y));
}

// Return the blurred mask along the x dimension.
fn roundedBoxShadowX(x: f32, y: f32, s: f32, corner: f32, halfSize: vec2f) -> f32 {
  let d = min(halfSize.y - corner - abs(y), 0);
  let c = halfSize.x - corner + sqrt(max(0, corner * corner - d * d));
  let integral = 0.5 + 0.5 * erf((x + vec2f(-c, c)) * (sqrt(0.5) / s));
  return integral.y - integral.x;
}

// Return the mask for the shadow of a box from lower to upper.
fn roundedBoxShadow(
  lower: vec2f,
  upper: vec2f,
  point: vec2f,
  sigma: f32,
  corners: vec4f
) -> f32 {
  // Center everything to make the math easier.
  let center = (lower + upper) * 0.5;
  let halfSize = (upper - lower) * 0.5;
  let p = point - center;

  // The signal is only non-zero in a limited range, so don't waste samples.
  let low = p.y - halfSize.y;
  let high = p.y + halfSize.y;
  let start = clamp(-3 * sigma, low, high);
  let end = clamp(3 * sigma, low, high);

  // Accumulate samples (we can get away with surprisingly few samples).
  let step = (end - start) / 4.0;
  var y = start + step * 0.5;
  var value: f32 = 0;

  for (var i = 0; i < 4; i++) {
    let corner = selectCorner(p.x, p.y, corners);
    value
      += roundedBoxShadowX(p.x, p.y - y, sigma, corner, halfSize)
      * gaussian(y, sigma) * step;
    y += step;
  }

  return value;
}

@vertex
fn vertexMain(input: VertexInput) -> VertexOutput {
  var output: VertexOutput;
  let r = data.rectangles[input.instance];
  let padding = 3 * r.sigma;
  let vertex = mix(
    r.position.xy - padding,
    r.position.xy + r.size + padding,
    input.position
  );

  output.position = vec4f(vertex / r.window * 2 - 1, 0, 1);
  output.position.y = -output.position.y;
  output.vertex = vertex;
  output.instance = input.instance;
  return output;
}

@fragment
fn fragmentMain(input: VertexOutput) -> @location(0) vec4f {
  let r = data.rectangles[input.instance];
  let alpha = r.color.a * roundedBoxShadow(
    r.position.xy,
    r.position.xy + r.size,
    input.vertex,
    r.sigma,
    r.corners
  );
  return vec4f(r.color.rgb, alpha);
}