import { Navigate, Outlet, useLocation } from "react-router-dom";
import { useEffect, useState } from "react";

import { getCurrentUser } from "../../lib/api/auth";
import { readAuthSession } from "./authSession";

export function RequireAuth() {
  const location = useLocation();
  const [isChecking, setIsChecking] = useState(() =>
    Boolean(readAuthSession()),
  );
  const [isAuthenticated, setIsAuthenticated] = useState(() =>
    Boolean(readAuthSession()),
  );

  useEffect(() => {
    const controller = new AbortController();
    getCurrentUser(controller.signal)
      .then(() => setIsAuthenticated(true))
      .catch(() => setIsAuthenticated(false))
      .finally(() => {
        if (!controller.signal.aborted) {
          setIsChecking(false);
        }
      });
    return () => controller.abort();
  }, []);

  if (isChecking) {
    return null;
  }

  if (!isAuthenticated) {
    return <Navigate to="/login" replace state={{ from: location }} />;
  }

  return <Outlet />;
}
