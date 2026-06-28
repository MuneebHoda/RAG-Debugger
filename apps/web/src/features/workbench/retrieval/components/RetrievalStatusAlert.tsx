import { AlertCircle } from "lucide-react";

export function RetrievalStatusAlert({ error }: { error: string | null }) {
  if (!error) return null;

  return (
    <div className="alert" role="alert">
      <AlertCircle aria-hidden="true" size={18} />
      <span>{error}</span>
    </div>
  );
}
