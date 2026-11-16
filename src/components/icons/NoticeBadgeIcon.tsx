import React from "react";

/** Colored by `currentColor`. */
export const NoticeBadgeIcon: React.FC<{ className?: string }> = ({
  className,
}) => (
  <svg className={className} viewBox="0 0 16 16" aria-hidden="true">
    <circle cx="8" cy="8" r="8" fill="currentColor" />
    <path
      d="M8 4v4.6"
      stroke="var(--color-background)"
      strokeWidth="2"
      strokeLinecap="round"
    />
    <path
      d="M8 11.8v.01"
      stroke="var(--color-background)"
      strokeWidth="2.2"
      strokeLinecap="round"
    />
  </svg>
);

/** Handy Flow: good news (a learned word). Colored by `currentColor`. */
export const CheckBadgeIcon: React.FC<{ className?: string }> = ({
  className,
}) => (
  <svg className={className} viewBox="0 0 16 16" aria-hidden="true">
    <circle cx="8" cy="8" r="8" fill="currentColor" />
    <path
      d="M4.6 8.3l2.3 2.3 4.5-4.8"
      fill="none"
      stroke="var(--color-background)"
      strokeWidth="1.9"
      strokeLinecap="round"
      strokeLinejoin="round"
    />
  </svg>
);
