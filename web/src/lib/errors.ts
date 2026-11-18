import type { AppError } from "./types";

export function normalizeError(error: unknown): AppError {
  if (
    error &&
    typeof error === "object" &&
    "message" in error &&
    typeof (error as { message?: unknown }).message === "string"
  ) {
    const candidate = error as Partial<AppError>;
    return {
      kind: candidate.kind ?? "other",
      message: candidate.message as string,
      retryable: candidate.retryable ?? false,
      field: candidate.field ?? null,
      actions: candidate.actions ?? []
    };
  }
  if (typeof error === "string") {
    return { kind: "other", message: error, retryable: false, field: null, actions: [] };
  }
  return {
    kind: "other",
    message: "Unexpected application error",
    retryable: false,
    field: null,
    actions: []
  };
}
