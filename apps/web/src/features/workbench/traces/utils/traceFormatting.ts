export function signedNumber(value: number) {
  return `${value >= 0 ? "+" : ""}${value.toFixed(2)}`;
}

export function requestErrorMessage(error: unknown) {
  return error instanceof Error ? error.message : "Request failed";
}
