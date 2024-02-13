![nothing graphics engine logo](./branding/cover.png)

# [nothing](https://nothing.graphics)

nothing but drawing. everything drawable engine. Powered by WebGPU and Rust.

## Install

```bash
npm install @wgpu/nothing
# yarn
yarn add @wgpu/nothing
# pnpm
pnpm install @wgpu/nothing
```

## Graphics engine

Engine for people eager to create, draw something on the canvas.

## Presentation engine

Engine for people eager to share, impress the world.

## Sheets engine

Engine for people eager to organize, make the knowledebase.

## Docs engine

Engine for people to make things real.

## All in one, performance fully optimized.

All in one, brand new experience you will have. don't imagine, try it now.

## Structure - Packages & Modules

![](./branding/project-map.png)

- [@nothing-sdk/skia](./packages/skia-backend) - Skia Backend
  - Skia backend with offscrean canvas support
- [nothing-skia](./packages/nothing-skia) - Skia Bridge
  - Skia Bridge with C++ for higher performance on interactive design platforms
- [@nothing-sdk/core](packages/nothing-core) - Skia Node Api
  - Skia SDK Wrapped with Beautifully designed API
- [@nothing-sdk/react-core](./packages/nothing-react-core) - React Core
  - React Core - Skia (core) Wrapped with React JSX. Best for only drawing
- [@nothing-sdk/react-compact](packages/nothing-react-compact) - React Compact
  - React Compact - Fully built graphics tools for drawing, editing. Best for light-weight graphics tool
- [@nothing-sdk/react](packages/nothing-react) - Nothing React
  - React SDK - Fully built, all-in-one graphics tools for drawing, editing and sharing
- [@nothing-sdk/vue-core](./packages/nothing-vue-core) - Vue Core
  - Vue Core - Skia (core) Wrapped with Vue JSX. Best for only drawing
- [@nothing-sdk/vue-compact](./packages/nothing-vue-compact) - Vue Compact
  - Vue Compact - Fully built graphics tools for drawing, editing. Best for light-weight graphics tool
- [@nothing-sdk/vue](packages/nothing-vue) - Nothing Vue
  - Vue SDK - Fully built, all-in-one graphics tools for drawing, editing and sharing
- [@nothing-sdk/commands](./packages/nothing-util-commands) - Command util
  - Graphics Commands Definitions and handling util. You can extend this to register your own commands.

## Building

```
git pull --recurse-submodules
yarn
yarn app
```

## Philosophy

- Ready to use - Comparing to figma or sketch, there must not be a limitations in usablity. this software will be enterprise standard opensourced UI Graphics tool.
- Opensourced - Yep it's here on github.
- Modularity - all features and functionality are provided in modular shape
- Performant - based on the new WebGPU, Nothing is performant and highly optimized, givving all native performance. cannot be compared to html5 canvas based applications.
- Reusability - all components are built to be used on other projects. you can build your entirely new projects from it.
- Extansibility - from extending a theme, to extending a
- Composable - Components are composable, same as code components. Now you can finally design as the code is.
- Largely built-in features - such as live collaboration support
- Fully customizable - from self hosting to extending functionality.

## Roadmap

- Canvas / Camera
  - Zoom
  - Pan
- primal
  - shapes
    - rectangle
    - circle
    - line
    - polygon
    - star
    - arrow
  - border
  - fill
    - solid
    - gradient
    - image
- shadow
- Glyph
  - span
  - text
  - paragraph
  - text-decoration
    - underline
    - overline
    - line-through
- Vector
  - svg
  - icon
- Image
  - image/jpeg
  - image/png
  - image/webp
  - image/gif
- Video
  - video/mp4
  - video/webm
- UI
  - layouts
    - flex
    - grid
    - stack
    - column
    - row
  - length
    - px
    - em
    - rem
    - vw
    - vh
    - %
- State
  - editing capabilities
- component/instance
- abstractions
  - masking
  - grouping
  - clipping
  - connections
- user interaction
  - pointer enter
  - pointer hover
  - pointer click
- I/O
  - commands
- Export
  - png
  - svg
  - pdf
- Optimizations
  - baking
  - image cache with prefered size

## References (Technical)

<details>
  <summary>
    Technical References  
  </summary>

- https://skia.org/
- https://api.skia.org/
- https://skia.org/docs/user/modules/canvaskit/
- https://groups.google.com/g/skia-discuss
- https://dbdiagram.io/
- https://draw.io/
- https://miro.com/
- http://figma.com/
- https://docs.microsoft.com/en-us/xamarin/xamarin-forms/user-interface/graphics/skiasharp/
- https://github.com/mono/SkiaSharp
- https://docs.microsoft.com/en-us/dotnet/api/skiasharp
- https://konvajs.org/
- https://github.com/rowsncolumns/grid/tree/master/packages/grid
</details>
