import { useEffect } from "react";
import { themeColor, type Ground } from "./ground";

/**
 * Puts the page on `ground`: its background, and the browser's `theme-color`.
 * Back to parchment, the default, once the screen is gone.
 */
export function useGround(ground: Ground) {
  useEffect(() => {
    apply(ground);
    return () => apply("parchment");
  }, [ground]);
}

function apply(ground: Ground) {
  document.documentElement.dataset.ground = ground;
  document.querySelector('meta[name="theme-color"]')?.setAttribute("content", themeColor(ground));
}
