import { Search } from "lucide-react";

import { WorkbenchEmptyState } from "../../../components/workbench/WorkbenchEmptyState";
import { WorkbenchPageHeader } from "../../../components/workbench/WorkbenchPageHeader";
import { WorkbenchWorkflowGuide } from "../../../components/workbench/WorkbenchWorkflowGuide";
import { EmbeddingPanel } from "./components/EmbeddingPanel";
import { RetrievalFiltersPanel } from "./components/RetrievalFiltersPanel";
import { RetrievalQueryPanel } from "./components/RetrievalQueryPanel";
import { RetrievalStatusAlert } from "./components/RetrievalStatusAlert";
import { SaveEvidenceToEvalPanel } from "../eval-lab/evidence/SaveEvidenceToEvalPanel";
import { useRetrievalWorkbench } from "./hooks/useRetrievalWorkbench";
import styles from "./RetrievalPage.module.css";
import { AnswerPanel, HitsPanel } from "./RetrievalResults";

export function RetrievalPage() {
  const workbench = useRetrievalWorkbench();

  return (
    <section className={styles.page} aria-labelledby="retrieval-title">
      <WorkbenchPageHeader
        description="Ask one diagnostic question, inspect ranked candidates, and verify which evidence can support an answer."
        section="Debug"
        title="Retrieval"
        titleId="retrieval-title"
      />

      <RetrievalStatusAlert error={workbench.error} />

      {!workbench.isLoadingSources && workbench.allDocuments.length === 0 ? (
        <WorkbenchEmptyState
          description="Retrieval needs indexed documents before it can rank evidence or assess answerability."
          icon={Search}
          primaryAction={{ label: "Open Corpus", to: "/app/sources" }}
          secondaryAction={{ label: "Load guided sample", to: "/app" }}
          title="Add a corpus before testing retrieval"
        />
      ) : null}

      <WorkbenchWorkflowGuide
        currentStep="retrieval"
        impact="Retrieval can rank broad candidates, but the Evidence Summary only answers when chunk body text passes answerability."
        nextAction={
          workbench.response
            ? {
                disabled: workbench.isSavingTrace,
                label: workbench.isSavingTrace
                  ? "Saving run"
                  : "Save run for diagnosis",
                onClick: () => void workbench.saveTrace(),
              }
            : { label: "Run retrieval", href: "#retrieval-question" }
        }
        purpose="Retrieval is the test step: ask one question, compare ranking modes, and inspect which chunks can actually support an answer."
      />

      <section className={styles.layout}>
        <RetrievalQueryPanel
          advancedControls={
            <>
              <EmbeddingPanel
                isIndexing={workbench.isIndexing}
                status={workbench.embeddingStatus}
                onIndex={() => void workbench.refreshEmbeddings()}
              />
              <RetrievalFiltersPanel
                documents={workbench.visibleDocuments}
                selectedDocumentIds={workbench.activeSelectedDocumentIds}
                selectedSourceIds={workbench.selectedSourceIds}
                sources={workbench.sources}
                onToggleDocument={workbench.toggleDocument}
                onToggleSource={workbench.toggleSource}
              />
            </>
          }
          documentCount={workbench.allDocuments.length}
          demoStatus={workbench.demoStatus}
          isLoadingSources={workbench.isLoadingSources}
          isQuerying={workbench.isQuerying}
          query={workbench.query}
          retrievalMode={workbench.retrievalMode}
          topK={workbench.topK}
          onQueryChange={workbench.setQuery}
          onRetrievalModeChange={workbench.setRetrievalMode}
          onSelectSuggestedQuery={workbench.selectSuggestedQuery}
          onSubmit={() => void workbench.submitQuery()}
          onTopKChange={workbench.setTopK}
        />

        <div className={styles.results}>
          <AnswerPanel
            isQuerying={workbench.isQuerying}
            isSavingTrace={workbench.isSavingTrace}
            response={workbench.response}
            onSaveTrace={() => void workbench.saveTrace()}
          />
          <HitsPanel
            isQuerying={workbench.isQuerying}
            response={workbench.response}
          />
          {workbench.response ? (
            <SaveEvidenceToEvalPanel
              candidateHits={workbench.response.hits.map((hit) => ({
                chunkId: hit.chunk.id,
                documentId: hit.document.id,
                label: hit.citation.label,
                rank: hit.rank,
                path: hit.document.path,
                sectionTitle: hit.chunk.section_title,
                snippet: hit.snippet,
                weak: hit.evidence_strength === "weak",
                duplicate: hit.duplicate_count > 1,
              }))}
              query={workbench.response.run.query}
              sourceNote={`Saved from retrieval run ${workbench.response.run.id.slice(0, 8)}.`}
              topK={workbench.response.run.top_k}
            />
          ) : null}
        </div>
      </section>
    </section>
  );
}
