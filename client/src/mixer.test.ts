import { describe, expect, test } from "vitest";
import { AMBIANCE_FILES, createMixer, DEFAULT_LEVELS, MIX, type AmbianceFile, type MixContext } from "./mixer";

/** A gain or other level: remembers where its last ramp is heading. */
class FakeParam {
  ramps: { to: number; at: number }[] = [];
  constructor(public value: number) {}
  cancelScheduledValues() {}
  setValueAtTime(value: number) {
    this.value = value;
  }
  linearRampToValueAtTime(to: number, at: number) {
    this.ramps.push({ to, at });
  }
  /** The level it ends at once its ramps are done. */
  get target() {
    return this.ramps.at(-1)?.to ?? this.value;
  }
}

type FakeNode = {
  kind: string;
  outputs: FakeNode[];
  connect: (node: FakeNode) => FakeNode;
  gain: FakeParam;
  buffer?: { name: AmbianceFile; duration: number };
  loop?: boolean;
  started?: { when: number; offset: number };
  stoppedAt?: number;
};

/** An AudioContext that records the graph it builds, at time `now`. */
function fakeContext() {
  const nodes: FakeNode[] = [];
  const node = (kind: string, extra: Partial<FakeNode> = {}): FakeNode => {
    const created: FakeNode = {
      kind,
      outputs: [],
      gain: new FakeParam(1),
      connect(other) {
        created.outputs.push(other);
        return other;
      },
      ...extra,
    };
    nodes.push(created);
    return created;
  };
  const destination = node("destination");
  const context = {
    currentTime: 10,
    destination,
    createGain: () => node("gain"),
    createDynamicsCompressor: () =>
      node("limiter", { threshold: new FakeParam(0), ratio: new FakeParam(1) } as Partial<FakeNode>),
    createBufferSource: () => {
      const source: FakeNode = node("source", {
        loop: false,
        start: (when = 0, offset = 0) => {
          source.started = { when, offset };
        },
        stop: (when = 0) => {
          source.stoppedAt = when;
        },
      } as Partial<FakeNode>);
      return source;
    },
  };
  return {
    context: context as unknown as MixContext,
    /** The sources playing `file`, in the order they were started. */
    sources: (file: AmbianceFile) => nodes.filter((n) => n.kind === "source" && n.buffer?.name === file),
    /** The kinds of node a node's sound goes through to the loudspeaker. */
    path(from: FakeNode) {
      const kinds: string[] = [];
      for (let at = from.outputs[0]; at; at = at.outputs[0]) kinds.push(at.kind);
      return kinds;
    },
    nodes,
  };
}


function mixerWith(loaded: AmbianceFile[] = AMBIANCE_FILES) {
  const fake = fakeContext();
  const buffer = (name: AmbianceFile) =>
    loaded.includes(name) ? ({ name, duration: 100 } as unknown as AudioBuffer) : undefined;
  const mixer = createMixer(fake.context, buffer, () => 0.5);
  return { ...fake, mixer };
}

/** The one gain a source plays through before its bus. */
const layerGain = (source: FakeNode) => source.outputs[0]!.gain;

describe("the Table device's mix", () => {
  test("the voice, the backgrounds and the sounds all end in one limiter", () => {
    const { mixer, sources, path } = mixerWith();
    mixer.setBackground("night");
    mixer.playSound("howl");

    const voice = path({ outputs: [mixer.voice as unknown as FakeNode] } as FakeNode);
    expect(voice.slice(-2)).toEqual(["limiter", "destination"]);
    for (const file of ["night-music", "night-wind", "wolf-howl"] as const) {
      expect(path(sources(file)[0]!).slice(-2)).toEqual(["limiter", "destination"]);
    }
  });

  test("each bus starts at its default level", () => {
    const { mixer, sources } = mixerWith();
    mixer.setBackground("day");
    mixer.playSound("sword");

    expect((mixer.voice as unknown as FakeNode).gain.value).toBe(DEFAULT_LEVELS.narrator);
    // A background goes through its layer's gain, then the ambiance bus.
    expect(sources("day-tavern")[0]!.outputs[0]!.outputs[0]!.gain.value).toBe(DEFAULT_LEVELS.ambiance);
    expect(sources("sword-slice")[0]!.outputs[0]!.gain.value).toBe(DEFAULT_LEVELS.effects);
  });
});

describe("the backgrounds", () => {
  test("the Night is suspense music and wind, looped, faded in", () => {
    const { mixer, sources } = mixerWith();

    mixer.setBackground("night");

    for (const file of ["night-music", "night-wind"] as const) {
      const [source] = sources(file);
      expect(source!.loop).toBe(true);
      // Started somewhere inside the loop, so a long Game doesn't always hear the same opening.
      expect(source!.started).toEqual({ when: 0, offset: 50 });
      expect(layerGain(source!).value).toBe(0);
      expect(layerGain(source!).ramps).toEqual([{ to: MIX.layers[file], at: 10 + MIX.fadeSeconds }]);
    }
    expect(sources("day-tavern")).toEqual([]);
  });

  test("dawn crossfades the Night into the tavern", () => {
    const { mixer, sources } = mixerWith();
    mixer.setBackground("night");

    mixer.setBackground("day");

    for (const file of ["night-music", "night-wind"] as const) {
      const [source] = sources(file);
      expect(layerGain(source!).target).toBe(0);
      expect(source!.stoppedAt).toBeGreaterThan(10 + MIX.fadeSeconds);
    }
    const [tavern] = sources("day-tavern");
    expect(layerGain(tavern!).value).toBe(0);
    expect(layerGain(tavern!).target).toBe(MIX.layers["day-tavern"]);
  });

  test("nightfall crossfades the tavern back into the Night, with fresh loops", () => {
    const { mixer, sources } = mixerWith();
    mixer.setBackground("night");
    mixer.setBackground("day");

    mixer.setBackground("night");

    expect(sources("day-tavern")[0]!.stoppedAt).toBeDefined();
    expect(sources("night-music")).toHaveLength(2);
    expect(layerGain(sources("night-music")[1]!).target).toBe(MIX.layers["night-music"]);
  });

  test("the same Phase again changes nothing", () => {
    const { mixer, sources } = mixerWith();
    mixer.setBackground("night");

    mixer.setBackground("night");

    expect(sources("night-music")).toHaveLength(1);
    expect(sources("night-music")[0]!.stoppedAt).toBeUndefined();
  });

  test("a layer that loads late joins the Phase it belongs to", () => {
    const loaded: AmbianceFile[] = ["night-music"];
    const { mixer, sources } = mixerWith(loaded);
    mixer.setBackground("night");
    expect(sources("night-wind")).toEqual([]);

    // The wind arrives: the same Phase again starts just what is missing.
    loaded.push("night-wind");
    mixer.setBackground("night");

    expect(sources("night-wind")).toHaveLength(1);
    expect(sources("night-music")).toHaveLength(1);
  });

  test("no background fades everything out", () => {
    const { mixer, sources } = mixerWith();
    mixer.setBackground("day");

    mixer.setBackground(null);

    expect(layerGain(sources("day-tavern")[0]!).target).toBe(0);
    expect(sources("day-tavern")[0]!.stoppedAt).toBeDefined();
  });
});

describe("ducking", () => {
  /** The gain the ambiance bus goes through: the one that dips under the voice. */
  function duckGain(fake: ReturnType<typeof mixerWith>) {
    fake.mixer.setBackground("day");
    return fake.sources("day-tavern")[0]!.outputs[0]!.outputs[0]!.outputs[0]!.gain;
  }

  test("the background dips while the Narrator speaks, quickly", () => {
    const fake = mixerWith();
    const duck = duckGain(fake);

    fake.mixer.duck(true);

    expect(duck.ramps).toEqual([{ to: MIX.duckLevel, at: 10 + MIX.duckInSeconds }]);
  });

  test("and comes back up, slowly, once the Narrator is silent", () => {
    const fake = mixerWith();
    const duck = duckGain(fake);
    fake.mixer.duck(true);

    fake.mixer.duck(false);

    expect(duck.ramps.at(-1)).toEqual({ to: 1, at: 10 + MIX.duckOutSeconds });
  });

  test("the voice and the sounds never dip", () => {
    const fake = mixerWith();
    fake.mixer.duck(true);
    fake.mixer.playSound("howl");

    expect((fake.mixer.voice as unknown as FakeNode).gain.ramps).toEqual([]);
    expect(fake.sources("wolf-howl")[0]!.outputs[0]!.gain.ramps).toEqual([]);
  });
});

describe("one-off sounds", () => {
  test("each Moment's sound plays its file once", () => {
    const { mixer, sources } = mixerWith();

    expect(mixer.playSound("howl")).toBe(true);
    expect(mixer.playSound("bottle")).toBe(true);
    expect(mixer.playSound("sword")).toBe(true);
    expect(mixer.playSound("gunshot")).toBe(true);

    for (const file of ["wolf-howl", "bottle-smash", "sword-slice", "gunshot"] as const) {
      expect(sources(file)).toHaveLength(1);
      expect(sources(file)[0]!.loop).toBe(false);
      expect(sources(file)[0]!.started).toBeDefined();
    }
  });

  test("a sound not loaded yet is skipped", () => {
    const { mixer, sources } = mixerWith([]);

    expect(mixer.playSound("howl")).toBe(false);
    expect(sources("wolf-howl")).toEqual([]);
  });
});
