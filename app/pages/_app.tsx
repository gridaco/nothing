import React from "react";
import { Suspense, StrictMode } from "react";

// enable SPA mode, supports react.Suspense; if you don't want to use Suspense, you can use NextJS' dynamic import instead. - on SSR mode
// though, this app does not benefit from SSR.
function SafeHydrate({ children }) {
  return (
    <div suppressHydrationWarning>
      {typeof window === "undefined" ? null : children}
    </div>
  );
}

function RootWebApp({ Component, pageProps }) {
  return (
    <SafeHydrate>
      <StrictMode>
        <Suspense fallback="Loading...">
          <Component {...pageProps} />
        </Suspense>
      </StrictMode>
    </SafeHydrate>
  );
}

export default RootWebApp;
