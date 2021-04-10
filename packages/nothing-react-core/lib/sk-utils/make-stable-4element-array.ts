import { useMemo } from "react";

export default function makeStable4ElementArray(
  value: Float32Array
): Float32Array;
export default function makeStable4ElementArray(
  value: Float32Array | undefined
): Float32Array | undefined;
export default function makeStable4ElementArray(
  value: Float32Array | undefined
): Float32Array | undefined {
  return value;

  // disabling this due to untested performance boost. this may slow things down. (don't know)
  // will this memo really be necessary here?
  return useMemo(
    () => value,
    //
    [value?.[0], value?.[1], value?.[2], value?.[3]]
  );
}
