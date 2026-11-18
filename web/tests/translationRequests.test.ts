import { beforeEach, describe, expect, it, vi } from "vitest";

const backend = vi.hoisted(() => ({
  startTranslation: vi.fn(),
  cancelTranslation: vi.fn()
}));

vi.mock("../src/lib/api", () => ({ getBackend: () => backend }));

import { useYiyuStore } from "../src/lib/store";

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((accept, fail) => {
    resolve = accept;
    reject = fail;
  });
  return { promise, resolve, reject };
}

describe("translation request ordering", () => {
  beforeEach(() => {
    vi.resetAllMocks();
    useYiyuStore.setState({ sessions: {}, notices: [] });
  });

  it("does not let an older failure overwrite a newer translation", async () => {
    const old = deferred<{ sessionId: string }>();
    const current = deferred<{ sessionId: string }>();
    backend.startTranslation.mockReturnValueOnce(old.promise).mockReturnValueOnce(current.promise);
    const first = useYiyuStore.getState().startTranslation("popup", "old");
    const second = useYiyuStore.getState().startTranslation("popup", "new");
    current.resolve({ sessionId: "new-session" });
    await second;
    old.reject({ kind: "cancelled", message: "cancelled" });
    await first;
    expect(useYiyuStore.getState().sessions.popup?.sessionId).toBe("new-session");
    expect(useYiyuStore.getState().notices).toHaveLength(0);
  });

  it("does not let an older start response replace a newer session", async () => {
    const old = deferred<{ sessionId: string }>();
    const current = deferred<{ sessionId: string }>();
    backend.startTranslation.mockReturnValueOnce(old.promise).mockReturnValueOnce(current.promise);
    const first = useYiyuStore.getState().startTranslation("popup", "old");
    const second = useYiyuStore.getState().startTranslation("popup", "new");
    current.resolve({ sessionId: "new-session" });
    await second;
    old.resolve({ sessionId: "old-session" });
    await first;
    expect(useYiyuStore.getState().sessions.popup?.sessionId).toBe("new-session");
  });

  it("does not let a delayed cancellation cancel a newer session", async () => {
    backend.startTranslation.mockResolvedValueOnce({ sessionId: "old-session" });
    await useYiyuStore.getState().startTranslation("popup", "old");
    const cancellation = deferred<void>();
    backend.cancelTranslation.mockReturnValueOnce(cancellation.promise);
    const cancel = useYiyuStore.getState().cancelTranslation("popup");
    backend.startTranslation.mockResolvedValueOnce({ sessionId: "new-session" });
    await useYiyuStore.getState().startTranslation("popup", "new");
    cancellation.resolve();
    await cancel;
    expect(useYiyuStore.getState().sessions.popup).toMatchObject({
      sessionId: "new-session",
      status: "preparing"
    });
  });
});
