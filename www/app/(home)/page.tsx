import Link from "next/link";
import { ArrowRight } from "lucide-react";

export default function HomePage() {
  return (
    <main className="landing">
      <section className="landing__hero">
        <span className="eyebrow">The Grida graphics engine</span>
        <h1>Nothing.</h1>
        <p>
          A Rust-first 2D graphics engine, built for native and WebAssembly
          surfaces.
        </p>
        <div className="landing__actions">
          <Link href="/docs" className="button button--primary">
            Documentation <ArrowRight size={15} aria-hidden="true" />
          </Link>
          <a
            href="https://github.com/gridaco/nothing"
            className="button"
            target="_blank"
            rel="noreferrer"
          >
            GitHub
          </a>
        </div>
      </section>
      <footer className="landing__footer">
        <span>n0 / nothing</span>
        <span>Rust · WebAssembly · WebGL</span>
      </footer>
    </main>
  );
}
