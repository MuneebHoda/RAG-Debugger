import { Navigate, Outlet, useLocation } from "react-router-dom";

import { readAuthSession } from "./authSession";

export function RequireAuth() {
  const location = useLocation();

  if (!readAuthSession()) {
    return <Navigate to="/login" replace state={{ from: location }} />;
  }

  return <Outlet />;
}
