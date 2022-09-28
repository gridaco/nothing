## Installation

```sh
yarn add @nothing.app/use-image
```

## Usage

```tsx
import React from "react";
import { Image } from "react-konva";
import useImage from "use-image";

const url =
  "https://bridged-service-static.s3-us-west-1.amazonaws.com/branding/bridged-logo-512.png";

function SimpleApp() {
  const [image] = useImage(url);

  // "image" will DOM image element or undefined

  return <Image image={image} />;
}

function ComplexApp() {
  // set crossOrigin of image as second argument
  const [image, status] = useImage(url, "Anonymous");

  // status can be "loading", "loaded" or "failed"

  return <Image image={image} />;
}
```

## Related

For Loading CORS Images, you can use https://cors.sh
