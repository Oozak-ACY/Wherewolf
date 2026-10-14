import { describe, expect, test } from "vitest";
import { qrCode } from "./qrCode";

describe("the QR code", () => {
  test("is a square grid with a quiet zone around it", () => {
    const qr = qrCode("https://wherewolf.app/l/AB12");

    // Version 2 holds a link this long: 25 modules, plus 4 on each side.
    expect(qr.size).toBe(25 + 8);
  });

  test("draws one unit square per dark module, offset by the quiet zone", () => {
    const qr = qrCode("https://wherewolf.app/l/AB12");

    // The top-left finder pattern starts with a dark module.
    expect(qr.path.startsWith("M4 4h1v1h-1z")).toBe(true);
  });

  test("changes with the link it encodes", () => {
    expect(qrCode("https://wherewolf.app/l/AB12").path).not.toBe(
      qrCode("https://wherewolf.app/l/CD34").path,
    );
  });
});
