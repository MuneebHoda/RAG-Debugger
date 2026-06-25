import {
  AlertTriangle,
  Boxes,
  Database,
  FileText,
  FlaskConical,
  GitBranch,
  Layers3,
  Search,
  Sparkles,
} from "lucide-react";
import { useEffect, useState } from "react";

import { ActionCard } from "../components/dashboard/ActionCard";
import { ActivityList } from "../components/dashboard/ActivityList";
import { EmptyState } from "../components/dashboard/EmptyState";
import { HealthScore } from "../components/dashboard/HealthScore";
import { MetricTile } from "../components/dashboard/MetricTile";
import { PipelineStep } from "../components/dashboard/PipelineStep";
import { ProgressBar } from "../components/dashboard/ProgressBar";
import { RiskList } from "../components/dashboard/RiskList";
import {
  getOverview,
  type OverviewMetric,
  type OverviewResponse,
} from "../lib/api/overview";
import type { DocumentProfile } from "../lib/api/sources";
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

export function OverviewPage() {
  const [overview, setOverview] = useState<OverviewResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const controller = new AbortController();
    getOverview(controller.signal)
      .then((nextOverview) => {
        setOverview(nextOverview);
        setError(null);
      })
      .catch((cause: unknown) => {
        if (!controller.signal.aborted) {
          setError(cause instanceof Error ? cause.message : "Request failed");
        }
      })
      .finally(() => {
        if (!controller.signal.aborted) {
          setLoading(false);
        }
      });

    return () => controller.abort();
  }, []);

  return (
    <section className={styles.page} aria-labelledby="overview-title">
      <header className={styles.header}>
        <div>
          <p>Corpus operations</p>
          <h1 id="overview-title">Mission Control</h1>
          <span>
            Monitor corpus readiness, retrieval quality, trace risk, and eval
            coverage from one operational surface.
          </span>
        </div>
        {overview ? (
          <div className={styles.snapshot}>
            <small>Snapshot</small>
            <strong>{formatDate(overview.generated_at)}</strong>
          </div>
        ) : null}
      </header>

      {error ? (
        <div className={styles.alert} role="alert">
          <AlertTriangle aria-hidden="true" size={18} />
          <span>{error}</span>
        </div>
      ) : null}

      {loading ? <LoadingSkeleton /> : null}

      {overview && !loading ? (
        <>
          {overview.health.status === "needs_documents" ? (
            <EmptyState action={overview.health.primary_action} />
          ) : null}

          <HealthScore health={overview.health} />

          <section
            className={styles.metrics}
            aria-label="Mission Control metrics"
          >
            {overview.metrics.map((metric) => (
              <MetricTile
                key={metric.id}
                metric={metric}
                icon={metricIcon(metric)}
              />
            ))}
          </section>

          <section className={styles.panel} aria-labelledby="pipeline-title">
            <div className={styles.panelHeading}>
              <div>
                <p>Pipeline board</p>
                <h2 id="pipeline-title">Ingest to report flow</h2>
              </div>
              <span>Operational sequence</span>
            </div>
            <div className={styles.pipeline}>
              {overview.pipeline.map((step) => (
                <PipelineStep key={step.id} step={step} />
              ))}
            </div>
          </section>

          <div className={styles.grid}>
            <RiskList issues={overview.issues} />

            <section
              className={styles.panel}
              aria-labelledby="next-actions-title"
            >
              <div className={styles.panelHeading}>
                <div>
                  <p>Next best actions</p>
                  <h2 id="next-actions-title">What to do now</h2>
                </div>
              </div>
              <div className={styles.actions}>
                {overview.actions.map((action) => (
                  <ActionCard key={action.id} action={action} />
                ))}
              </div>
            </section>
          </div>

          <div className={styles.grid}>
            <ActivityList activity={overview.recent_activity} />
            <DocumentMix overview={overview} />
          </div>
        </>
      ) : null}
    </section>
  );
}

function LoadingSkeleton() {
  return (
    <div className={styles.loading} aria-label="Loading overview">
      <span />
      <span />
      <span />
    </div>
  );
}

function DocumentMix({ overview }: { overview: OverviewResponse }) {
  const warningMetric = overview.metrics.find(
    (metric) => metric.id === "warnings",
  );
  const totalDocuments = overview.document_mix.reduce(
    (total, profile) => total + profile.count,
    0,
  );

  return (
    <section className={styles.panel} aria-labelledby="document-mix-title">
      <div className={styles.panelHeading}>
        <div>
          <p>Document mix</p>
          <h2 id="document-mix-title">Profiles and quality signals</h2>
        </div>
        <span>{totalDocuments} documents</span>
      </div>

      <div className={styles.profileList}>
        {overview.document_mix.length === 0 ? (
          <div className={styles.noProfiles}>
            <Layers3 aria-hidden="true" size={18} />
            No document profiles detected yet.
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

      <div className={styles.qualityStrip}>
        <span>
          <Search aria-hidden="true" size={15} />
          {overview.embedding_status.indexed_chunks}/
          {overview.embedding_status.total_chunks} embedded
        </span>
        <span>
          <AlertTriangle aria-hidden="true" size={15} />
          {warningMetric?.value ?? "0"} warnings
        </span>
      </div>
    </section>
  );
}

function metricIcon(metric: OverviewMetric) {
  return metricIcons[metric.id] ?? Database;
}

function formatDate(value: string) {
  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  }).format(new Date(value));
}

function profileLabel(profile: DocumentProfile) {
  return profile
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}
