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
      <header className={styles.pageHeader}>
        <div>
          <p className={styles.eyebrow}>Build</p>
          <h1 id="retrieval-title">Test retrieval</h1>
          <p>Ask one question and inspect the evidence CorpusLab would use.</p>
        </div>
      </header>

      <RetrievalStatusAlert error={workbench.error} />

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
          isLoadingSources={workbench.isLoadingSources}
          isQuerying={workbench.isQuerying}
          query={workbench.query}
          retrievalMode={workbench.retrievalMode}
          topK={workbench.topK}
          onQueryChange={workbench.setQuery}
          onRetrievalModeChange={workbench.setRetrievalMode}
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
