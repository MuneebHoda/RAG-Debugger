import { describe, expect, it } from "vitest";

import { ApiError, readJsonResponse, readTextResponse } from "./client";

describe("API client errors", () => {
  it("parses the structured backend error envelope", async () => {
    const body = JSON.stringify({
      error: {
        code: "not_found",
        message: "not found: trace",
      },
    });

    const error = await capturedError(new Response(body, { status: 404 }));

    expect(error).toMatchObject({
      message: "not found: trace",
      status: 404,
      code: "not_found",
      body,
    });
  });

  it("does not expose a plain-text response as the user message", async () => {
    const error = await capturedError(
      new Response("upstream proxy details", { status: 502 }),
    );

    expect(error.message).toBe("Request failed with 502");
    expect(error.body).toBe("upstream proxy details");
    expect(error.code).toBeUndefined();
  });

  it("uses a stable fallback for an empty response", async () => {
    const error = await capturedError(new Response(null, { status: 500 }));

    expect(error.message).toBe("Request failed with 500");
    expect(error.body).toBe("");
  });

  it("reads successful text responses without JSON parsing", async () => {
    const markdown = "# Audit report\n";

    await expect(
      readTextResponse(new Response(markdown, { status: 200 })),
    ).resolves.toBe(markdown);
  });

  it("uses the same structured errors for text responses", async () => {
    const body = JSON.stringify({
      error: { code: "validation_error", message: "export is blocked" },
    });

    await expect(
      readTextResponse(new Response(body, { status: 422 })),
    ).rejects.toMatchObject({
      message: "export is blocked",
      status: 422,
      code: "validation_error",
      body,
    });
  });
});

async function capturedError(response: Response): Promise<ApiError> {
  try {
    await readJsonResponse(response);
    throw new Error("expected request to fail");
  } catch (error) {
    expect(error).toBeInstanceOf(ApiError);
    return error as ApiError;
  }
}
