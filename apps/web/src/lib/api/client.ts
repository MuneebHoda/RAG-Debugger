export const API_BASE_URL =
  import.meta.env.VITE_API_BASE_URL ?? "http://127.0.0.1:8080";

export class ApiError extends Error {
  constructor(
    message: string,
    public readonly status: number,
    public readonly body: string,
    public readonly code?: string,
  ) {
    super(message);
    this.name = "ApiError";
  }
}

export async function requestJson<T>(
  path: string,
  init: RequestInit = {},
  okStatuses: number[] = [200],
): Promise<T> {
  const response = await fetch(apiUrl(path), {
    credentials: "include",
    ...init,
  });
  return readJsonResponse<T>(response, okStatuses);
}

export function jsonRequest(
  method: "POST" | "PATCH" | "DELETE",
  body: unknown,
  signal?: AbortSignal,
): RequestInit {
  return {
    method,
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
    signal,
  };
}

export async function readJsonResponse<T>(
  response: Response,
  okStatuses: number[] = [200],
): Promise<T> {
  if (!okStatuses.includes(response.status)) {
    const text = await response.text();
    const error = parseErrorEnvelope(text);
    throw new ApiError(
      error.message ?? `Request failed with ${response.status}`,
      response.status,
      text,
      error.code,
    );
  }

  return response.json() as Promise<T>;
}

type ApiErrorEnvelope = {
  error?: {
    code?: unknown;
    message?: unknown;
  };
};

function parseErrorEnvelope(body: string): {
  code?: string;
  message?: string;
} {
  if (!body) return {};

  try {
    const envelope = JSON.parse(body) as ApiErrorEnvelope;
    return {
      code:
        typeof envelope.error?.code === "string"
          ? envelope.error.code
          : undefined,
      message:
        typeof envelope.error?.message === "string"
          ? envelope.error.message
          : undefined,
    };
  } catch {
    return {};
  }
}

function apiUrl(path: string): string {
  if (path.startsWith("http://") || path.startsWith("https://")) {
    return path;
  }

  return `${API_BASE_URL}${path.startsWith("/") ? path : `/${path}`}`;
}
