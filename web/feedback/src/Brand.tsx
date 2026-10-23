// Baylee's mark: two cards, one laid over the other, on the accent. Drawn
// here, so nothing is fetched for it; `public/favicon.svg` is the same
// drawing.

import { useId } from "react";

/** The product's name, in the bar, on the sign-in form and in the window's title. */
export const BRAND = "Baylee reports";

export function BrandMark({ size = 28 }: { size?: number }) {
  // A gradient is found by id; two marks on a page must not share one.
  const gradient = `brand-${useId().replace(/[^a-zA-Z0-9_-]/g, "")}`;
  return (
    <svg
      className="brand-mark"
      viewBox="0 0 32 32"
      width={size}
      height={size}
      aria-hidden="true"
      focusable="false"
    >
      <defs>
        <linearGradient id={gradient} x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#4d68f0" />
          <stop offset="1" stopColor="#7449d8" />
        </linearGradient>
      </defs>
      <rect width="32" height="32" rx="8" fill={`url(#${gradient})`} />
      <rect
        x="8"
        y="8"
        width="11"
        height="15"
        rx="2"
        fill="none"
        stroke="#fff"
        strokeOpacity="0.6"
        strokeWidth="1.6"
        transform="rotate(-10 13.5 15.5)"
      />
      <rect x="13" y="9.5" width="11" height="15" rx="2" fill="#fff" transform="rotate(8 18.5 17)" />
    </svg>
  );
}
