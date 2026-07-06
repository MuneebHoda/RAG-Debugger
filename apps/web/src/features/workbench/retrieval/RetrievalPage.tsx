import { Search } from "lucide-react";

import { WorkbenchEmptyState } from "../../../components/workbench/WorkbenchEmptyState";
import { WorkbenchPageHeader } from "../../../components/workbench/WorkbenchPageHeader";
import { EmbeddingPanel } from "./components/EmbeddingPanel";
import { RetrievalFiltersPanel } from "./components/RetrievalFiltersPanel";
import { RetrievalQueryPanel } from "./components/RetrievalQueryPanel";
import { RetrievalStatusAlert } from "./components/RetrievalStatusAlert";
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
        </div>
      </section>
    </section>
  );
}
