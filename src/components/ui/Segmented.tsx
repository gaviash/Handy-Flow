import React, { useRef } from "react";

export function Segmented<T extends string>({
  options,
  value,
  onChange,
  ariaLabel,
}: {
  options: { value: T; label: string }[];
  value: T;
  onChange: (value: T) => void;
  ariaLabel: string;
}) {
  const group = useRef<HTMLDivElement>(null);
  return (
    <div
      ref={group}
      role="radiogroup"
      aria-label={ariaLabel}
      className="inline-flex flex-wrap gap-1 rounded-[10px] bg-surface p-1"
    >
      {options.map((option, index) => (
        <button
          key={option.value}
          type="button"
          role="radio"
          aria-checked={value === option.value}
          tabIndex={value === option.value ? 0 : -1}
          className={`rounded-[7px] px-3 py-1 text-sm transition-colors duration-150 ease-glide ${value === option.value ? "bg-accent text-on-accent font-semibold" : "text-ink-2 hover:bg-raised"}`}
          onClick={() => onChange(option.value)}
          onKeyDown={(event) => {
            const step = ["ArrowRight", "ArrowDown"].includes(event.key)
              ? 1
              : ["ArrowLeft", "ArrowUp"].includes(event.key)
                ? -1
                : 0;
            if (!step && event.key !== "Home" && event.key !== "End") return;
            event.preventDefault();
            const next =
              event.key === "Home"
                ? 0
                : event.key === "End"
                  ? options.length - 1
                  : (index + step + options.length) % options.length;
            group.current
              ?.querySelectorAll<HTMLButtonElement>("button")
              [next]?.focus();
            onChange(options[next].value);
          }}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}
