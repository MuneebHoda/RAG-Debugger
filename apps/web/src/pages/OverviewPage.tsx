import {
  AlertTriangle,
  Boxes,
  Database,
  FileText,
  FlaskConical,
  GitBranch,
  Layers3,
  Sparkles,
} from "lucide-react";
import { useQuery } from "@tanstack/react-query";

import { ActivityList } from "../components/dashboard/ActivityList";
import { MetricTile } from "../components/dashboard/MetricTile";
import { ProgressBar } from "../components/dashboard/ProgressBar";
import { RiskList } from "../components/dashboard/RiskList";
import { SetupChecklist } from "../features/workbench/home/SetupChecklist";
import {
  getOverview,
  type OverviewMetric,
  type OverviewResponse,
} from "../lib/api/overview";
import type { DocumentProfile } from "../lib/api/sources";
import { formatDateTime } from "../lib/dateTime";
import styles from "./OverviewPage.module.css";

const metricIcons: Record<string, typeof FileText> = {
  sources: Database,
  documents: FileText,
  chunks: Boxes,
  embeddings: Sparkles,
  traces: GitBranch,
  evals: FlaskConical,
  warnings: AlertTriangle,
};

const primaryMetricIds = new Set([
  "documents",
  "embeddings",
  "traces",
  "evals",
]);

export function OverviewPage() {
  const overviewQuery = useQuery({
    queryKey: ["overview"],
    queryFn: ({ signal }) => getOverview(signal),
  });

  return (
    <section className={styles.page} aria-labelledby="overview-title">
      <header className={styles.header}>
        <div>
          <p>Workspace</p>
          <h1 id="overview-title">Home</h1>
          <span>Your next action, current quality, and recent work.</span>
        </div>
        {overviewQuery.data ? (
          <div className={styles.snapshot}>
            <small>Updated</small>
            <strong>{formatDateTime(overviewQuery.data.generated_at)}</strong>
          </div>
        ) : null}
      </header>

      {overviewQuery.isError ? (
        <div className={styles.alert} role="alert">
          <AlertTriangle aria-hidden="true" size={18} />
          <span>
            {overviewQuery.error instanceof Error
              ? overviewQuery.error.message
              : "Could not load workspace status."}
          </span>
          <button type="button" onClick={() => void overviewQuery.refetch()}>
            Retry
          </button>
        </div>
      ) : null}

      {overviewQuery.isLoading ? <LoadingSkeleton /> : null}

      {overviewQuery.data ? (
        <HomeContent overview={overviewQuery.data} />
      ) : null}
    </section>
  );
}

function HomeContent({ overview }: { overview: OverviewResponse }) {
  const primaryMetrics = overview.metrics.filter((metric) =>
    primaryMetricIds.has(metric.id),
  );

  return (
    <>
      <SetupChecklist overview={overview} />

      <section className={styles.healthBand} aria-label="Workspace health">
        <div className={styles.healthScore}>
          <strong>{overview.health.score}</strong>
          <span>/100</span>
        </div>
        <div>
          <p>{statusLabel(overview.health.status)}</p>
          <h2>{overview.health.summary}</h2>
        </div>
      </section>

      <section className={styles.metrics} aria-label="Workspace metrics">
        {primaryMetrics.map((metric) => (
          <MetricTile
            key={metric.id}
            metric={metric}
            icon={metricIcon(metric)}
          />
        ))}
      </section>

      <div className={styles.grid}>
        <RiskList issues={overview.issues} />
        <ActivityList activity={overview.recent_activity} />
      </div>

      <details className={styles.systemDetails}>
        <summary>System details</summary>
        <div className={styles.detailsGrid}>
          <DocumentMix overview={overview} />
          <section className={styles.panel}>
            <div className={styles.panelHeading}>
              <div>
                <p>Corpus totals</p>
                <h2>Storage and quality signals</h2>
              </div>
            </div>
            <div className={styles.secondaryMetrics}>
              {overview.metrics
                .filter((metric) => !primaryMetricIds.has(metric.id))
                .map((metric) => (
                  <MetricTile
                    key={metric.id}
                    metric={metric}
                    icon={metricIcon(metric)}
                  />
                ))}
            </div>
          </section>
        </div>
      </details>
    </>
  );
}

function LoadingSkeleton() {
  return (
    <div className={styles.loading} aria-label="Loading workspace">
      <span />
      <span />
      <span />
    </div>
  );
}

function DocumentMix({ overview }: { overview: OverviewResponse }) {
  const totalDocuments = overview.document_mix.reduce(
    (total, profile) => total + profile.count,
    0,
  );

  return (
    <section className={styles.panel} aria-labelledby="document-mix-title">
      <div className={styles.panelHeading}>
        <div>
          <p>Document mix</p>
          <h2 id="document-mix-title">Profiles</h2>
        </div>
        <span>{totalDocuments} documents</span>
      </div>

      <div className={styles.profileList}>
        {overview.document_mix.length === 0 ? (
          <div className={styles.noProfiles}>
            <Layers3 aria-hidden="true" size={18} /> No profiles yet
          </div>
        ) : (
          overview.document_mix.map((profile) => (
            <div key={profile.profile} className={styles.profileRow}>
              <div>
                <strong>{profileLabel(profile.profile)}</strong>
                <span>{profile.count} documents</span>
              </div>
              <ProgressBar
                value={profile.percentage}
                tone="good"
                label={`${profileLabel(profile.profile)} profile share`}
              />
              <small>{Math.round(profile.percentage * 100)}%</small>
            </div>
          ))
        )}
      </div>
    </section>
  );
}

function metricIcon(metric: OverviewMetric) {
  return metricIcons[metric.id] ?? Database;
}

function statusLabel(status: OverviewResponse["health"]["status"]) {
  return {
    ready: "Ready",
    needs_indexing: "Indexing needed",
    needs_eval_coverage: "Quality coverage needed",
    needs_documents: "Documents needed",
  }[status];
}

function profileLabel(profile: DocumentProfile) {
  return profile
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}
