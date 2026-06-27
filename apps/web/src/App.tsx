import { QueryClientProvider } from "@tanstack/react-query";
import { lazy, Suspense } from "react";
import { Navigate, Route, Routes } from "react-router-dom";

import { queryClient } from "./app/queryClient";
import { LoginPage } from "./features/auth/LoginPage";
import { RequireAuth } from "./features/auth/RequireAuth";
import { SignupPage } from "./features/auth/SignupPage";
import { FeaturesPage } from "./features/marketing/FeaturesPage";
import { PricingPage } from "./features/marketing/PricingPage";
import { AuthLayout } from "./layouts/AuthLayout";
import { MarketingLayout } from "./layouts/MarketingLayout";
import { WorkbenchLayout } from "./layouts/WorkbenchLayout";
import { EvalsPage } from "./pages/EvalsPage";
import { DocumentDetailPage } from "./pages/DocumentDetailPage";
import { DatasetDetailPage } from "./pages/DatasetDetailPage";
import { ExperimentDetailPage } from "./pages/ExperimentDetailPage";
import { OverviewPage } from "./pages/OverviewPage";
import { ReportsPage } from "./pages/ReportsPage";
import { RetrievalPage } from "./pages/RetrievalPage";
import { SettingsPage } from "./pages/SettingsPage";
import { SourcesPage } from "./pages/SourcesPage";
import { TracesPage } from "./pages/TracesPage";
import { TraceDetailPage } from "./pages/TraceDetailPage";

const LandingPage = lazy(
  () => import("./features/marketing/landing/LandingPage"),
);

export function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <Routes>
        <Route element={<MarketingLayout />}>
          <Route
            index
            element={
              <Suspense
                fallback={<div aria-label="Loading CorpusLab" role="status" />}
              >
                <LandingPage />
              </Suspense>
            }
          />
          <Route path="features" element={<FeaturesPage />} />
          <Route path="pricing" element={<PricingPage />} />
        </Route>

        <Route element={<AuthLayout />}>
          <Route path="login" element={<LoginPage />} />
          <Route path="signup" element={<SignupPage />} />
        </Route>

        <Route path="app" element={<RequireAuth />}>
          <Route element={<WorkbenchLayout />}>
            <Route index element={<OverviewPage />} />
            <Route path="sources" element={<SourcesPage />} />
            <Route
              path="sources/:documentId"
              element={<DocumentDetailPage />}
            />
            <Route path="retrieval" element={<RetrievalPage />} />
            <Route path="traces" element={<TracesPage />} />
            <Route path="traces/:traceId" element={<TraceDetailPage />} />
            <Route path="evals" element={<EvalsPage />} />
            <Route
              path="evals/datasets/:datasetId"
              element={<DatasetDetailPage />}
            />
            <Route
              path="evals/experiments/:experimentId"
              element={<ExperimentDetailPage />}
            />
            <Route path="reports" element={<ReportsPage />} />
            <Route path="settings" element={<SettingsPage />} />
          </Route>
        </Route>

        <Route
          path="sources"
          element={<Navigate to="/app/sources" replace />}
        />
        <Route
          path="retrieval"
          element={<Navigate to="/app/retrieval" replace />}
        />
        <Route path="evals" element={<Navigate to="/app/evals" replace />} />
        <Route path="traces" element={<Navigate to="/app/traces" replace />} />
        <Route
          path="reports"
          element={<Navigate to="/app/reports" replace />}
        />
        <Route
          path="settings"
          element={<Navigate to="/app/settings" replace />}
        />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </QueryClientProvider>
  );
}
