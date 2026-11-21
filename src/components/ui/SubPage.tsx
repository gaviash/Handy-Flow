import React, { useCallback, useEffect, useRef, useState } from "react";
import { ChevronLeft } from "lucide-react";

/** Which sub-page of a settings page is open, and the card each one came
 * from, so going back returns the focus there. Local state: leaving the tab
 * unmounts the page and closes it. */
export function useSubPage<T extends string>(initial: T | null = null) {
  const [open, setOpen] = useState<T | null>(initial);
  const cards = useRef(new Map<T, HTMLButtonElement | null>());
  const last = useRef<T | null>(null);
  useEffect(() => {
    if (open === null && last.current) cards.current.get(last.current)?.focus();
  }, [open]);
  return {
    open,
    show: useCallback((id: T) => {
      last.current = id;
      setOpen(id);
    }, []),
    back: useCallback(() => setOpen(null), []),
    cardRef: (id: T) => (el: HTMLButtonElement | null) => {
      cards.current.set(id, el);
    },
  };
}

export const SubPage: React.FC<{
  parent: string;
  title: string;
  description?: React.ReactNode;
  onBack: () => void;
  children: React.ReactNode;
}> = ({ parent, title, description, onBack, children }) => {
  const heading = useRef<HTMLHeadingElement>(null);
  useEffect(() => heading.current?.focus(), []);
  return (
    <div className="mx-auto w-full max-w-3xl space-y-5 pb-4">
      <div className="space-y-2">
        <button
          type="button"
          onClick={onBack}
          className="-ms-1 inline-flex cursor-pointer items-center gap-1 rounded px-1 text-sm text-ink-2 hover:text-text focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-logo-primary"
        >
          <ChevronLeft
            aria-hidden="true"
            size={16}
            className="rtl:rotate-180"
          />
          {parent}
        </button>
        <h1 ref={heading} tabIndex={-1} className="page-title outline-none">
          {title}
        </h1>
        {description && <p className="text-sm text-ink-2">{description}</p>}
      </div>
      {children}
    </div>
  );
};
