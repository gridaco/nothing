# @nothing.app/react-core

nothing graphics engine core library for react. runs on skia 2d graphics engine

## Installation

```
yarn add @nothing.app/react-core
```

## Usage

**skia core api** -- prefix SK

```tsx
<Stage>
    <SKRect x={0} y={0} width={0} hight={0}>
</Stage>
```

**core graphics api** -- prefix CG

```tsx
<Stage>
    <CGRect x={0} y={0} width={0} hight={0}>
</Stage>
```

**nothing graphics api (reflect ui based)** -- prefix None

```tsx
<Stage>
    <Rect x={0} y={0} width={0} hight={0}>
</Stage>
```

## Disclaimer

Some of the code are from below. there are all great projects, we recommand you to take a look

1. [react-canvaskit](https://github.com/udevbe/react-canvaskit) (MIT) 2021.2
2. [noya](https://github.com/noya-app/noya) (MIT) 2021.4
