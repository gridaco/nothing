export function BrandMark({ className = "" }: { className?: string }) {
  return (
    <span className={`brand-mark ${className}`} aria-hidden="true">
      <span className="brand-mark__slash brand-mark__slash--left" />
      <span className="brand-mark__slash brand-mark__slash--right" />
      <span className="brand-mark__dot brand-mark__dot--nw" />
      <span className="brand-mark__dot brand-mark__dot--ne" />
      <span className="brand-mark__dot brand-mark__dot--sw" />
      <span className="brand-mark__dot brand-mark__dot--se" />
    </span>
  );
}

export function Brand() {
  return (
    <span className="brand">
      <BrandMark />
      <span>Nothing</span>
    </span>
  );
}
