// A QR code drawn in the client, so the join link never leaves this browser.

import { encode } from "uqr";

/** The light margin scanners need around the code, in modules. */
const QUIET_ZONE = 4;

export type QrCode = {
  /** Width and height, in modules, quiet zone included. */
  size: number;
  /** An SVG path of the dark modules, one unit per module. */
  path: string;
};

export function qrCode(text: string): QrCode {
  const { size, data } = encode(text, { border: QUIET_ZONE });
  let path = "";
  data.forEach((row, y) =>
    row.forEach((dark, x) => {
      if (dark) path += `M${x} ${y}h1v1h-1z`;
    }),
  );
  return { size, path };
}
