import React from "react";

// The brand name is not translated.
const [FIRST, SECOND] = ["Handy", "Flow"] as const;

/** "Handy" in Figtree 800, "Flow" in Instrument Serif italic, in the
 * palette's accent text color. Text, not paths: it reads as one name to
 * screen readers through the label. */
export const HandyFlowWordmark: React.FC<{
  size?: number;
  className?: string;
}> = ({ size = 26, className = "" }) => (
  <span
    role="img"
    aria-label="Handy Flow"
    className={`inline-flex items-baseline gap-[0.1em] whitespace-nowrap leading-none select-none ${className}`}
    style={{ fontSize: size }}
  >
    <b aria-hidden className="font-sans font-extrabold tracking-[-0.035em]">
      {FIRST}
    </b>
    <i
      aria-hidden
      className="font-serif text-[1.18em] font-normal italic"
      style={{ color: "var(--wordmark-flow, var(--accent-text))" }}
    >
      {SECOND}
    </i>
  </span>
);

export default HandyFlowWordmark;
