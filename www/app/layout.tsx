import { RootProvider } from "fumadocs-ui/provider/next";
import type { Metadata } from "next";
import { Geist, Geist_Mono } from "next/font/google";
import "./global.css";

const geistSans = Geist({
  subsets: ["latin"],
  variable: "--font-geist-sans",
});

const geistMono = Geist_Mono({
  subsets: ["latin"],
  variable: "--font-geist-mono",
});

export const metadata: Metadata = {
  metadataBase: new URL("https://nothing.graphics"),
  title: {
    default: "Nothing — the Grida graphics engine",
    template: "%s · Nothing",
  },
  description:
    "A Rust-first 2D graphics engine with a WebAssembly SDK for the web.",
  applicationName: "Nothing",
  openGraph: {
    type: "website",
    siteName: "Nothing",
    url: "https://nothing.graphics",
    title: "Nothing — the Grida graphics engine",
    description:
      "A Rust-first 2D graphics engine with a WebAssembly SDK for the web.",
  },
  twitter: {
    card: "summary_large_image",
    title: "Nothing — the Grida graphics engine",
    description:
      "A Rust-first 2D graphics engine with a WebAssembly SDK for the web.",
  },
};

export default function Layout({ children }: LayoutProps<"/">) {
  return (
    <html
      lang="en"
      className={`${geistSans.variable} ${geistMono.variable}`}
      suppressHydrationWarning
    >
      <body className="flex min-h-screen flex-col">
        <RootProvider
          theme={{
            defaultTheme: "light",
            enableSystem: false,
          }}
        >
          {children}
        </RootProvider>
      </body>
    </html>
  );
}
