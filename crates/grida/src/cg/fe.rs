use super::*;
use serde::Deserialize;

/// Represents filter effects inspired by SVG `<filter>` primitives.
///
/// See also:
/// - https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feDropShadow
/// - https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feGaussianBlur
#[derive(Debug, Clone)]
pub enum FilterEffect {
    /// Drop shadow filter: offset + blur + spread + color
    DropShadow(FeShadow),

    /// Inner shadow filter: offset + blur + spread + color
    /// the shadow is clipped to the shape
    InnerShadow(FeShadow),

    /// Layer blur filter
    LayerBlur(FeLayerBlur),

    /// Background blur filter
    /// A background blur effect, similar to CSS `backdrop-filter: blur(...)`
    BackdropBlur(FeBackdropBlur),

    /// Noise effect
    Noise(FeNoiseEffect),

    /// Liquid glass effect
    LiquidGlass(FeLiquidGlass),
}

impl FilterEffect {
    /// Returns whether this effect is active
    pub fn active(&self) -> bool {
        match self {
            FilterEffect::DropShadow(s) => s.active,
            FilterEffect::InnerShadow(s) => s.active,
            FilterEffect::LayerBlur(b) => b.active,
            FilterEffect::BackdropBlur(b) => b.active,
            FilterEffect::Noise(n) => n.active,
            FilterEffect::LiquidGlass(g) => g.active,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FilterShadowEffect {
    DropShadow(FeShadow),
    InnerShadow(FeShadow),
}

impl FilterShadowEffect {
    /// Returns whether this shadow effect is active
    pub fn active(&self) -> bool {
        match self {
            FilterShadowEffect::DropShadow(s) => s.active,
            FilterShadowEffect::InnerShadow(s) => s.active,
        }
    }
}

impl From<FilterShadowEffect> for FilterEffect {
    fn from(val: FilterShadowEffect) -> Self {
        match val {
            FilterShadowEffect::DropShadow(shadow) => FilterEffect::DropShadow(shadow),
            FilterShadowEffect::InnerShadow(shadow) => FilterEffect::InnerShadow(shadow),
        }
    }
}

/// A shadow (box-shadow) filter effect (`<feDropShadow>` + spread radius)
///
/// Grida's standard shadow effect that supports
/// - css box-shadow
/// - css text-shadow
/// - path-shadow (non-box) that supports css box-shadow properties
/// - fully compatible with feDropShadow => [FeShadow] (but no backwards compatibility, since spread is not supported by SVG)
///
/// See also:
/// - https://developer.mozilla.org/en-US/docs/Web/SVG/Element/feDropShadow
/// - https://developer.mozilla.org/en-US/docs/Web/CSS/box-shadow
/// - https://www.figma.com/plugin-docs/api/Effect/#dropshadoweffect
/// - https://api.flutter.dev/flutter/painting/BoxShadow-class.html
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FeShadow {
    /// Horizontal shadow offset in px
    pub dx: f32,

    /// Vertical shadow offset in px
    pub dy: f32,

    /// Blur radius (`stdDeviation` in SVG)
    pub blur: f32,

    /// Spread radius in px
    /// applies outset (or inset if inner) to the src rect
    pub spread: f32,

    /// Shadow color (includes alpha)
    pub color: CGColor,

    /// Whether this effect is active
    pub active: bool,
}

/// Liquid glass effect parameters
///
/// A physically-based glass effect with refraction, chromatic aberration, and Fresnel reflections.
/// This effect is designed for rectangular container elements (similar to HTML `div` with `border-radius`).
///
/// ## Key Properties
///
/// - **`light_intensity`**: Controls transmission/transparency of the glass (0.0 = opaque, 1.0 = fully transparent)
/// - **`refraction`**: Index of refraction that controls how much light bends (1.0 = no bend/air, 1.5 = typical glass)
/// - **`depth`**: Glass thickness that creates the 3D curved surface effect
/// - **`dispersion`**: Chromatic aberration strength (color separation at edges)
/// - **`blur_radius`**: Background blur radius for frosted glass appearance
///
/// ## Limitations
///
/// This effect only works with rectangular shapes. It uses Signed Distance Fields (SDFs) to generate
/// the 3D glass surface, which requires continuous distance information not available for arbitrary paths.
///
/// ## See also:
/// - Shader implementation: `src/shaders/liquid_glass_effect.sksl`
/// - Documentation: `src/shaders/liquid_glass_effect.md`
/// - Example: `examples/golden_liquid_glass.rs`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FeLiquidGlass {
    /// Controls transmission/transparency [0.0-1.0]
    /// Higher values = more see-through glass
    pub light_intensity: f32,

    /// Light angle in degrees (reserved for future use)
    pub light_angle: f32,

    /// Refraction strength [0.0-1.0]
    /// 0.0 = no refraction, 0.5 = typical glass, 1.0 = maximum refraction
    /// Internally mapped to IOR range [1.0-2.0]
    pub refraction: f32,

    /// Glass thickness/depth for 3D surface effect in pixels [1.0+]
    /// Controls the curvature height of the glass surface
    /// Higher values create more pronounced lens curvature and stronger refraction
    /// Typical values: 20-100 pixels
    pub depth: f32,

    /// Chromatic aberration strength [0.0-1.0]
    /// Controls color separation at edges (rainbow effect)
    pub dispersion: f32,

    /// Blur radius for frosted glass effect [0.0+] in pixels
    /// Applied via Skia's native blur before refraction shader
    pub blur_radius: f32,

    /// Whether this effect is active
    pub active: bool,
}

impl Default for FeLiquidGlass {
    fn default() -> Self {
        Self {
            light_intensity: 0.9,
            light_angle: 45.0,
            refraction: 0.8,  // Normalized [0.0-1.0], maps to IOR [1.0-2.0]
            depth: 20.0,      // Absolute pixels [1.0+], typical values: 20-100
            dispersion: 0.5,  // Chromatic aberration strength [0.0-1.0]
            blur_radius: 4.0, // Blur radius in pixels
            active: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
pub enum FeBlur {
    Gaussian(FeGaussianBlur),
    Progressive(FeProgressiveBlur),
}

/// Layer blur effect wrapper with active flag
#[derive(Debug, Clone, PartialEq)]
pub struct FeLayerBlur {
    pub blur: FeBlur,
    pub active: bool,
}

/// Backdrop blur effect wrapper with active flag
#[derive(Debug, Clone, PartialEq)]
pub struct FeBackdropBlur {
    pub blur: FeBlur,
    pub active: bool,
}

/// A standalone blur filter effect (`<feGaussianBlur>`)
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct FeGaussianBlur {
    /// Blur radius (`stdDeviation` in SVG)
    pub radius: f32,
}

/// Progressive blur effect with gradient-based blur intensity.
///
/// Applies a blur that varies in intensity along a gradient direction, creating a smooth
/// transition from sharp to blurred. The blur intensity is controlled by two points (start/end)
/// and their corresponding blur radii.
///
/// ## Coordinate System: Normalized Node-Local Space
///
/// The `start` and `end` coordinates use **normalized node-local space** with [`Alignment`],
/// identical to how linear gradient coordinates work:
///
/// - `Alignment(0.0, 0.0)` = center of the node
/// - `Alignment(-1.0, -1.0)` = top-left corner
/// - `Alignment(1.0, 1.0)` = bottom-right corner
/// - `Alignment(0.0, -1.0)` = top edge center
/// - `Alignment(0.0, 1.0)` = bottom edge center
///
/// Values can extend beyond `[-1.0, 1.0]` to define gradients that start/end outside the node bounds.
///
/// This normalized system ensures the effect scales correctly with the node regardless of its
/// actual pixel dimensions, and works consistently across different rendering contexts.
///
/// ### Important: Canvas vs Node-Local Coordinates
///
/// **In production (node effects)**: Coordinates are **node-local** and automatically scaled
/// to the node's dimensions. A vertical blur from top to bottom is simply:
/// ```rust
/// use grida::cg::prelude::*;
///
/// let blur = FeProgressiveBlur {
///     start: Alignment(0.0, -1.0),  // Top center (node-local)
///     end: Alignment(0.0, 1.0),      // Bottom center (node-local)
///     radius: 0.0,
///     radius2: 40.0,
/// };
/// ```
/// This works for **any node size** - the coordinates are relative to the node's bounds.
///
/// **In standalone examples (canvas-space)**: When applying progressive blur directly to a
/// canvas without a node (as in `golden_progressive_blur.rs`), you must manually calculate
/// and convert canvas-space pixel coordinates to normalized coordinates:
/// ```rust
/// use grida::cg::prelude::*;
/// // For a 150×300 rectangle at canvas position (125, 50):
/// // Node bounds: x=125..275, y=50..350
/// // Node center: (200, 200)
/// // Node half-size: (75, 150)
///
/// // To blur from top to bottom in node-local space:
/// let blur = FeProgressiveBlur {
///     start: Alignment(0.0, -1.0),  // Top edge of node
///     end: Alignment(0.0, 1.0),      // Bottom edge of node  
///     radius: 0.0,
///     radius2: 40.0,
/// };
/// ```
///
/// ### Example: Vertical Gradient Blur
///
/// ```rust
/// use grida::cg::prelude::*;
///
/// // Blur from sharp at top to maximum at bottom (works for any node size)
/// let blur = FeProgressiveBlur {
///     start: Alignment(0.0, -1.0),  // Top edge (sharp)
///     end: Alignment(0.0, 1.0),      // Bottom edge (max blur)
///     radius: 0.0,    // No blur at start
///     radius2: 40.0,  // 40px blur at end
/// };
/// ```
///
/// ### Example: Diagonal Gradient Blur
///
/// ```rust
/// use grida::cg::prelude::*;
/// // Blur from top-left to bottom-right
/// let blur = FeProgressiveBlur {
///     start: Alignment(-1.0, -1.0),  // Top-left corner (sharp)
///     end: Alignment(1.0, 1.0),      // Bottom-right corner (max blur)
///     radius: 0.0,
///     radius2: 30.0,
/// };
/// ```
///
/// ### Example: Horizontal Gradient Blur
///
/// ```rust
/// use grida::cg::prelude::*;
/// // Blur from left edge to right edge
/// let blur = FeProgressiveBlur {
///     start: Alignment(-1.0, 0.0),  // Left edge center (sharp)
///     end: Alignment(1.0, 0.0),      // Right edge center (max blur)
///     radius: 0.0,
///     radius2: 25.0,
/// };
/// ```
///
/// ## Production Usage: Automatic Scaling
///
/// When used as a `LayerEffects` blur on scene graph nodes, the normalized coordinates
/// are automatically scaled to the node's pixel dimensions:
///
/// ```ignore
/// // For a 200×400 pixel rectangle node:
/// FeProgressiveBlur {
///     start: Alignment(0.0, -1.0),  // Top edge in node-local space
///     end: Alignment(0.0, 1.0),      // Bottom edge in node-local space
///     radius: 0.0,
///     radius2: 40.0,
/// }
/// // The gradient runs vertically through the rectangle regardless of its position on canvas.
/// // Alignment(0.0, -1.0) evaluates to y=0 (top), Alignment(0.0, 1.0) evaluates to y=400 (bottom).
/// ```
///
/// The node can be positioned anywhere on the canvas, and the blur gradient will correctly
/// follow the node's transform (translation, rotation, scale).
///
/// ## Gradient Direction & Interpolation
///
/// The blur intensity is determined by projecting each pixel onto the gradient vector from
/// `start` to `end`:
/// - Pixels at the start point have blur radius = `radius`  
/// - Pixels at the end point have blur radius = `radius2`
/// - Pixels between are linearly interpolated
///
/// ## Implementation Details
///
/// Uses a two-pass separable Gaussian blur for performance (~30× faster than 2D blur):
/// 1. Horizontal pass: blur along X-axis with gradient-varying radius
/// 2. Vertical pass: blur along Y-axis with gradient-varying radius
///
/// This is mathematically equivalent to 2D Gaussian blur while being significantly faster.
///
/// ## See Also
///
/// - [`Alignment`] - The coordinate system used for start/end points
/// - Shader implementation: `src/shaders/progressive_blur_horizontal.sksl`, `progressive_blur_vertical.sksl`
/// - Documentation: `src/shaders/progressive_blur.md`  
/// - Examples: `examples/golden_progressive_blur.rs`, `examples/golden_progressive_blur_backdrop.rs`
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct FeProgressiveBlur {
    /// Gradient start point in normalized node-local space
    ///
    /// Uses [`Alignment`] coordinates where `(0.0, 0.0)` is the center,
    /// `(-1.0, -1.0)` is top-left, and `(1.0, 1.0)` is bottom-right.
    pub start: Alignment,

    /// Gradient end point in normalized node-local space
    ///
    /// Uses [`Alignment`] coordinates where `(0.0, 0.0)` is the center,
    /// `(-1.0, -1.0)` is top-left, and `(1.0, 1.0)` is bottom-right.
    pub end: Alignment,

    /// Blur radius at gradient start point (pixels)
    pub radius: f32,

    /// Blur radius at gradient end point (pixels)
    pub radius2: f32,
}

// ============================================================================
// Conversions for cleaner effect construction
// ============================================================================

/// Convert f32 radius to FeGaussianBlur
impl From<f32> for FeGaussianBlur {
    fn from(radius: f32) -> Self {
        Self { radius }
    }
}

/// Wrap FeGaussianBlur in FeBlur enum
impl From<FeGaussianBlur> for FeBlur {
    fn from(blur: FeGaussianBlur) -> Self {
        FeBlur::Gaussian(blur)
    }
}

/// Convert f32 radius to FeBlur
/// Convenience for: radius → FeGaussianBlur → FeBlur
impl From<f32> for FeBlur {
    fn from(radius: f32) -> Self {
        FeBlur::from(FeGaussianBlur::from(radius))
    }
}

/// Wrap FeProgressiveBlur in FeBlur enum
impl From<FeProgressiveBlur> for FeBlur {
    fn from(blur: FeProgressiveBlur) -> Self {
        FeBlur::Progressive(blur)
    }
}

/// Wrap FeBlur in FeLayerBlur with default active=true
impl From<FeBlur> for FeLayerBlur {
    fn from(blur: FeBlur) -> Self {
        Self { blur, active: true }
    }
}

/// Convert f32 radius directly to FeLayerBlur
/// Convenience for: radius → FeGaussianBlur → FeBlur → FeLayerBlur
impl From<f32> for FeLayerBlur {
    fn from(radius: f32) -> Self {
        FeLayerBlur::from(FeBlur::from(FeGaussianBlur::from(radius)))
    }
}

/// Wrap FeBlur in FeBackdropBlur with default active=true
impl From<FeBlur> for FeBackdropBlur {
    fn from(blur: FeBlur) -> Self {
        Self { blur, active: true }
    }
}

/// Convert f32 radius directly to FeBackdropBlur
/// Convenience for: radius → FeGaussianBlur → FeBlur → FeBackdropBlur
impl From<f32> for FeBackdropBlur {
    fn from(radius: f32) -> Self {
        FeBackdropBlur::from(FeBlur::from(FeGaussianBlur::from(radius)))
    }
}

/// Wrap FeLayerBlur in FilterEffect
impl From<FeLayerBlur> for FilterEffect {
    fn from(blur: FeLayerBlur) -> Self {
        FilterEffect::LayerBlur(blur)
    }
}

/// Wrap FeBackdropBlur in FilterEffect
impl From<FeBackdropBlur> for FilterEffect {
    fn from(blur: FeBackdropBlur) -> Self {
        FilterEffect::BackdropBlur(blur)
    }
}

/// Wrap FeShadow in FilterEffect as DropShadow
impl From<FeShadow> for FilterEffect {
    fn from(shadow: FeShadow) -> Self {
        FilterEffect::DropShadow(shadow)
    }
}

/// Wrap FeNoiseEffect in FilterEffect
impl From<FeNoiseEffect> for FilterEffect {
    fn from(noise: FeNoiseEffect) -> Self {
        FilterEffect::Noise(noise)
    }
}

/// Wrap FeLiquidGlass in FilterEffect
impl From<FeLiquidGlass> for FilterEffect {
    fn from(glass: FeLiquidGlass) -> Self {
        FilterEffect::LiquidGlass(glass)
    }
}

/// Convert f32 radius directly to FilterEffect::LayerBlur
/// Convenience for: radius → FeGaussianBlur → FeBlur → FeLayerBlur → FilterEffect
impl From<f32> for FilterEffect {
    fn from(radius: f32) -> Self {
        let gaussian = FeGaussianBlur::from(radius);
        let blur = FeBlur::from(gaussian);
        let layer_blur = FeLayerBlur::from(blur);
        FilterEffect::LayerBlur(layer_blur)
    }
}

/// Coloring strategy for noise effects.
///
/// All types use the same underlying Perlin noise pattern controlled by
/// `noise_size` and `density`, differing only in how colors are applied.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NoiseEffectColors {
    /// Single-color noise pattern.
    ///
    /// Renders noise pixels in the specified color with alpha blending.
    /// The `density` parameter controls how much of the noise is visible.
    ///
    /// # Example
    /// ```ignore
    /// Mono { color: CGColor::from_rgba(0, 0, 0, 64) } // 25% opacity black noise
    /// ```
    Mono {
        /// Color of the noise pixels (includes alpha)
        color: CGColor,
    },

    /// Dual-color noise with distinct background and pattern colors.
    ///
    /// Renders `color2` as a base layer, then applies `color1` noise pattern on top.
    /// The `density` parameter controls the pattern coverage.
    ///
    /// # Example
    /// ```ignore
    /// Duo {
    ///     color1: CGColor::from_rgba(255, 0, 0, 255),  // red pattern
    ///     color2: CGColor::from_rgba(255, 255, 255, 128) // semi-transparent white base
    /// }
    /// ```
    Duo {
        /// Pattern color (applied where noise is visible)
        color1: CGColor,
        /// Background color (base layer)
        color2: CGColor,
    },

    /// Multi-color RGB noise using the raw Perlin output.
    ///
    /// Renders the RGB colors directly from the noise shader, controlled by
    /// both `density` (pattern visibility) and `opacity` (overall transparency).
    ///
    /// # Example
    /// ```ignore
    /// Multi { opacity: 0.5 } // 50% opacity RGB noise
    /// ```
    Multi {
        /// Overall transparency (0..1)
        opacity: f32,
    },
}

/// Procedural noise effect with configurable pattern and coloring.
///
/// # Noise Generation
///
/// Uses Skia's fractal Perlin noise with the following pipeline:
/// 1. Generate base noise at specified frequency and octaves
/// 2. Convert to alpha mask via luminance-to-alpha color filter
/// 3. Apply density-based LUT cutoff to control visibility
/// 4. Apply type-specific coloring (Mono/Duo/Multi)
///
/// # Parameters
///
/// - **`noise_size`**: Controls grain size (smaller = finer grains)
/// - **`density`**: Controls pattern visibility (0 = sparse, 1 = dense)
/// - **`num_octaves`**: Fractal detail level (more = finer detail)
/// - **`seed`**: Random seed for reproducibility
///
/// # SVG Equivalents
///
/// - Noise generation: `<feTurbulence type="fractalNoise">`
/// - Alpha conversion: `<feColorMatrix type="luminanceToAlpha">`
/// - Density control: `<feComponentTransfer>` with table values
/// - Color application: `<feFlood>` + `<feComposite operator="in">`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FeNoiseEffect {
    /// Controls noise grain size (lower = finer grains)
    pub noise_size: f32,
    /// Controls pattern visibility via LUT cutoff (0..1)
    pub density: f32,
    /// Number of octaves for fractal detail
    pub num_octaves: i32,
    /// Random seed for reproducibility
    pub seed: f32,
    /// Coloring strategy
    pub coloring: NoiseEffectColors,
    /// Whether this effect is active
    pub active: bool,
    /// Blend mode for compositing the noise effect with fills
    pub blend_mode: BlendMode,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct LayerEffects {
    /// single layer blur is supported per layer
    /// layer blur is applied after all other effects
    pub blur: Option<FeLayerBlur>,
    /// single backdrop blur is supported per layer
    pub backdrop_blur: Option<FeBackdropBlur>,
    /// multiple shadows are supported per layer (drop shadow, inner shadow)
    pub shadows: Vec<FilterShadowEffect>,
    /// single liquid glass effect is supported per layer (only fully supported with rectangular shapes)
    pub glass: Option<FeLiquidGlass>,
    /// multiple noise effects are supported per layer
    pub noises: Vec<FeNoiseEffect>,
}

impl LayerEffects {
    /// Create a new LayerEffects (alias for default)
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true when there are no effects at all (no shadows, blur,
    /// backdrop blur, glass, or noise). Used for fast-path dispatch
    /// to skip the effects pipeline entirely for simple nodes.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.blur.is_none()
            && self.backdrop_blur.is_none()
            && self.glass.is_none()
            && self.shadows.is_empty()
            && self.noises.is_empty()
    }

    /// Set layer blur effect
    pub fn blur(mut self, blur: impl Into<FeBlur>) -> Self {
        self.blur = Some(FeLayerBlur::from(blur.into()));
        self
    }

    /// Set backdrop blur effect
    pub fn backdrop_blur(mut self, blur: impl Into<FeBlur>) -> Self {
        self.backdrop_blur = Some(FeBackdropBlur::from(blur.into()));
        self
    }

    /// Add a drop shadow effect
    pub fn drop_shadow(mut self, shadow: impl Into<FeShadow>) -> Self {
        self.shadows
            .push(FilterShadowEffect::DropShadow(shadow.into()));
        self
    }

    /// Add multiple drop shadow effects
    pub fn drop_shadows(mut self, shadows: Vec<FeShadow>) -> Self {
        for shadow in shadows {
            self.shadows.push(FilterShadowEffect::DropShadow(shadow));
        }
        self
    }

    /// Add an inner shadow effect
    pub fn inner_shadow(mut self, shadow: impl Into<FeShadow>) -> Self {
        self.shadows
            .push(FilterShadowEffect::InnerShadow(shadow.into()));
        self
    }

    /// Add multiple inner shadow effects
    pub fn inner_shadows(mut self, shadows: Vec<FeShadow>) -> Self {
        for shadow in shadows {
            self.shadows.push(FilterShadowEffect::InnerShadow(shadow));
        }
        self
    }

    /// Add a noise effect
    pub fn noise(mut self, noise: impl Into<FeNoiseEffect>) -> Self {
        self.noises.push(noise.into());
        self
    }

    /// Add multiple noise effects
    pub fn noises(mut self, noises: Vec<FeNoiseEffect>) -> Self {
        self.noises.extend(noises);
        self
    }

    /// Set liquid glass effect
    pub fn glass(mut self, glass: impl Into<FeLiquidGlass>) -> Self {
        self.glass = Some(glass.into());
        self
    }

    /// Returns true if opacity must be isolated in a separate save_layer
    /// because effects (shadows, blur, glass, backdrop blur) render outside
    /// the opacity wrapper and should appear at full alpha.
    ///
    /// When false, opacity can be safely folded into a parent save_layer
    /// or the paint alpha, eliminating a GPU surface allocation.
    #[inline]
    pub fn needs_opacity_isolation(&self) -> bool {
        // Drop/inner shadows render outside opacity — they should appear at
        // full opacity even when the shape content is semi-transparent.
        if self.shadows.iter().any(|s| s.active()) {
            return true;
        }
        // Layer blur wraps everything including content — opacity inside
        // blur vs outside blur produces different results.
        if self.blur.as_ref().is_some_and(|b| b.active) {
            return true;
        }
        // Backdrop blur and glass read from content behind the node
        // and render outside the opacity wrapper.
        if self.backdrop_blur.as_ref().is_some_and(|b| b.active) {
            return true;
        }
        if self.glass.as_ref().is_some_and(|g| g.active) {
            return true;
        }
        false
    }

    /// Returns true if this layer has any active effects that are expensive
    /// to paint (shadows, blurs, noise, glass).  Simple fill/stroke-only
    /// nodes return false.
    pub fn has_expensive_effects(&self) -> bool {
        if self.blur.as_ref().is_some_and(|b| b.active) {
            return true;
        }
        // Note: backdrop_blur is context-dependent and excluded from
        // compositing by the promotion heuristic, so we don't count it here.
        if self.shadows.iter().any(|s| s.active()) {
            return true;
        }
        if self.glass.as_ref().is_some_and(|g| g.active) {
            return true;
        }
        if self.noises.iter().any(|n| n.active) {
            return true;
        }
        false
    }

    /// Convert a list of filter effects into a layer effects object.
    /// if multiple effects that is not supported, the last effect will be used.
    pub fn from_array(effects: Vec<FilterEffect>) -> Self {
        let mut layer_effects = Self::default();
        for effect in effects {
            match effect {
                FilterEffect::LayerBlur(blur) => layer_effects.blur = Some(blur),
                FilterEffect::BackdropBlur(blur) => layer_effects.backdrop_blur = Some(blur),
                FilterEffect::LiquidGlass(glass) => layer_effects.glass = Some(glass),
                FilterEffect::DropShadow(shadow) => layer_effects
                    .shadows
                    .push(FilterShadowEffect::DropShadow(shadow)),
                FilterEffect::InnerShadow(shadow) => layer_effects
                    .shadows
                    .push(FilterShadowEffect::InnerShadow(shadow)),
                FilterEffect::Noise(noise) => layer_effects.noises.push(noise),
            }
        }
        layer_effects
    }

    #[deprecated(note = "will be removed")]
    pub fn fallback_first_any_effect(&self) -> Option<FilterEffect> {
        if let Some(blur) = &self.blur {
            return Some(FilterEffect::LayerBlur(blur.clone()));
        }
        if let Some(backdrop_blur) = &self.backdrop_blur {
            return Some(FilterEffect::BackdropBlur(backdrop_blur.clone()));
        }
        if !self.shadows.is_empty() {
            return Some(self.shadows.last().unwrap().clone().into());
        }
        None
    }
}
