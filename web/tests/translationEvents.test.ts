import { describe, expect, it } from "vitest";
import { applyTranslationEvent } from "../src/lib/store";
import type { TranslationEvent, TranslationSession } from "../src/lib/types";

function baseSession(): TranslationSession {
  return {
    sessionId: "session-a",
    surface: "popup",
    sourceText: "hello",
    status: "preparing",
    output: "",
    direction: null,
    error: null,
    durationMs: null,
    historySaved: null,
    persistenceError: null
  };
}

function stateWith(session: TranslationSession) {
  return {
    sessions: { popup: session },
    historyQuery: ""
  } as never;
}

function event(overrides: Partial<TranslationEvent>): TranslationEvent {
  return {
    sessionId: "session-a",
    surface: "popup",
    state: "streaming",
    sourceText: "hello",
    text: null,
    direction: null,
    error: null,
    durationMs: null,
    historySaved: null,
    persistenceError: null,
    ...overrides
  };
}

describe("translation latest-wins reducer", () => {
  it("appends chunks for the active session", () => {
    const session = { ...baseSession(), status: "streaming" as const, output: "你" };
    const next = applyTranslationEvent(stateWith(session), event({ text: "好" }));
    expect((next.sessions as Record<string, TranslationSession>).popup.output).toBe("你好");
  });

  it("ignores a stale completed event", () => {
    const session = baseSession();
    const next = applyTranslationEvent(
      stateWith(session),
      event({ sessionId: "session-old", state: "completed", text: "stale" })
    );
    expect(next).toEqual({});
  });

  it("starts a fresh popup session from a global shortcut without retaining the old result", () => {
    const old = { ...baseSession(), status: "completed" as const, output: "旧译文" };
    const next = applyTranslationEvent(
      stateWith(old),
      event({ sessionId: "session-new", state: "preparing", sourceText: "new text" })
    );
    const popup = (next.sessions as Record<string, TranslationSession>).popup;
    expect(popup.sessionId).toBe("session-new");
    expect(popup.sourceText).toBe("new text");
    expect(popup.output).toBe("");
    expect(applyTranslationEvent(stateWith(popup), event({ state: "completed", text: "stale" }))).toEqual({});
  });

  it("keeps partial output when the stream fails", () => {
    const session = {
      ...baseSession(),
      status: "streaming" as const,
      output: "部分译文"
    };
    const next = applyTranslationEvent(
      stateWith(session),
      event({
        state: "failed",
        error: {
          kind: "network",
          message: "connection lost",
          retryable: true,
          field: null,
          actions: []
        }
      })
    );
    expect((next.sessions as Record<string, TranslationSession>).popup.output).toBe(
      "部分译文"
    );
    expect(
      (next.sessions as Record<string, TranslationSession>).popup.status
    ).toBe("failed");
  });

  it("does not let a late completion overwrite a cancelled session", () => {
    const session = { ...baseSession(), status: "cancelled" as const };
    const next = applyTranslationEvent(
      stateWith(session),
      event({ state: "completed", text: "late result" })
    );
    expect(next).toEqual({});
  });
});
