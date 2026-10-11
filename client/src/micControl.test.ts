import { describe, expect, test } from "vitest";
import { createMicControl, type MicMode } from "./micControl";

function setup(mode: MicMode = "pushToTalk") {
  const applied: boolean[] = [];
  const mic = createMicControl(mode, (live) => applied.push(live));
  return { mic, applied };
}

describe("push-to-talk", () => {
  test("starts muted and is live only while held", () => {
    const { mic } = setup();
    expect(mic.live()).toBe(false);

    mic.press();
    expect(mic.live()).toBe(true);
    mic.release();
    expect(mic.live()).toBe(false);
  });

  test("toggling does nothing: only holding talks", () => {
    const { mic } = setup();

    mic.toggle();

    expect(mic.live()).toBe(false);
  });
});

describe("open mic", () => {
  test("starts live and a tap mutes or unmutes it", () => {
    const { mic } = setup("open");
    expect(mic.live()).toBe(true);

    mic.toggle();
    expect(mic.live()).toBe(false);
    mic.toggle();
    expect(mic.live()).toBe(true);
  });

  test("releasing (a blur, the page hidden) does not mute it", () => {
    const { mic } = setup("open");

    mic.release();

    expect(mic.live()).toBe(true);
  });
});

describe("switching modes", () => {
  test("to open mic starts live, even if it was muted before", () => {
    const { mic } = setup("open");
    mic.toggle();
    mic.setMode("pushToTalk");

    mic.setMode("open");

    expect(mic.mode()).toBe("open");
    expect(mic.live()).toBe(true);
  });

  test("back to push-to-talk mutes at once", () => {
    const { mic } = setup("open");

    mic.setMode("pushToTalk");

    expect(mic.mode()).toBe("pushToTalk");
    expect(mic.live()).toBe(false);
  });
});

describe("with the Narrator speaking", () => {
  test("a line mutes the mic in both modes", () => {
    for (const mode of ["pushToTalk", "open"] as const) {
      const { mic } = setup(mode);
      mic.press();

      mic.narratorStarted();

      expect(mic.live()).toBe(false);
    }
  });

  test("not holding: the mic stays muted after the line", () => {
    const { mic } = setup();

    mic.narratorStarted();
    mic.narratorEnded();

    expect(mic.live()).toBe(false);
  });

  test("holding through the line: live after it", () => {
    const { mic } = setup();
    mic.press();

    mic.narratorStarted();
    mic.narratorEnded();

    expect(mic.live()).toBe(true);
  });

  test("pressing during the line: live only after it", () => {
    const { mic } = setup();
    mic.narratorStarted();

    mic.press();
    expect(mic.live()).toBe(false);

    mic.narratorEnded();
    expect(mic.live()).toBe(true);
  });

  test("the Player's own wish to talk is kept apart from the Narrator muting", () => {
    const { mic } = setup("open");

    mic.narratorStarted();

    expect(mic.live()).toBe(false);
    expect(mic.wantsToTalk()).toBe(true);
  });

  test("open mic muted by the Player stays muted after the line", () => {
    const { mic } = setup("open");
    mic.toggle();

    mic.narratorStarted();
    mic.narratorEnded();

    expect(mic.live()).toBe(false);
  });
});

test("applies the mic only when whether it is live changes", () => {
  const { mic, applied } = setup();

  mic.press();
  mic.press();
  mic.narratorStarted();
  mic.narratorStarted();
  mic.narratorEnded();
  mic.release();
  mic.release();

  expect(applied).toEqual([true, false, true, false]);
});
