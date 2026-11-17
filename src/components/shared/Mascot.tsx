import React from "react";
import HandyMascot from "../icons/HandyMascot";

interface MascotProps {
  /** Height in px. */
  size: number;
  /** Wave once on arrival, or high-five (re-key it to play again). */
  animation?: "wave" | "slap" | null;
}

// The design's 357×400 image box, kept so sizes and the wave's pivot match it.
const BOX_ASPECT = 357 / 400;

/** Handy's mascot glove, optionally animated. */
export const Mascot: React.FC<MascotProps> = ({ size, animation = null }) => (
  <span
    style={{ width: size * BOX_ASPECT, height: size }}
    className={`relative block origin-[50%_90%] rotate-[-8deg] select-none ${animation === "wave" ? "animate-wave" : animation === "slap" ? "animate-slap" : ""}`}
  >
    <HandyMascot className="absolute start-[12.6%] top-[7.25%] h-[85.25%] w-auto" />
  </span>
);

// Each empty page greets once per launch: a wave every visit would grow old.
const greeted = new Set<string>();

/** The mascot for an empty page: it waves the first time that page is seen. */
export const EmptyMascot: React.FC<{ page: string; size?: number }> = ({
  page,
  size = 44,
}) => {
  const [wave] = React.useState(() => !greeted.has(page));
  React.useEffect(() => {
    greeted.add(page);
  }, [page]);
  return (
    <span aria-hidden className="mx-auto mb-2 block w-fit">
      <Mascot size={size} animation={wave ? "wave" : null} />
    </span>
  );
};
