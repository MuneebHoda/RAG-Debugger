import { ArrowRight } from "lucide-react";
import { AnimatePresence, m, useReducedMotion } from "motion/react";
import { useState } from "react";
import { Link } from "react-router-dom";

import { productTour, type ProductTourItem } from "./landingData";
import {
  motionTransition,
  quickTransition,
  revealVariants,
  viewportOnce,
} from "./motion";
import { useRovingTabs } from "./useRovingTabs";
import styles from "./ProductTour.module.css";

const productTourIds = productTour.map((item) => item.id);

export function ProductTour() {
  const [activeId, setActiveId] = useState<ProductTourItem["id"]>("dashboard");
  const reducedMotion = useReducedMotion();
  const { handleTabKeyDown, registerTab } = useRovingTabs({
    ids: productTourIds,
    onSelect: setActiveId,
  });
  const activeItem =
    productTour.find((item) => item.id === activeId) ?? productTour[0];

  return (
    <m.section
      className={styles.section}
      initial={reducedMotion ? false : "hidden"}
      variants={revealVariants}
      viewport={viewportOnce}
      whileInView="visible"
      aria-labelledby="product-tour-title"
    >
      <header className={styles.heading}>
        <p>Inside the workbench</p>
        <h2 id="product-tour-title">
          One evidence system. Every quality decision.
        </h2>
        <span>
          Move from corpus health to retrieval diagnosis and release gates
          without losing the source, chunk, score, or decision trail.
        </span>
      </header>

      <div className={styles.tabs} aria-label="Product tour" role="tablist">
        {productTour.map((item, index) => (
          <button
            aria-controls="product-tour-panel"
            aria-selected={item.id === activeItem.id}
            className={
              item.id === activeItem.id ? styles.activeTab : styles.tab
            }
            id={`product-tab-${item.id}`}
            key={item.id}
            ref={registerTab(index)}
            role="tab"
            tabIndex={item.id === activeItem.id ? 0 : -1}
            type="button"
            onClick={() => setActiveId(item.id)}
            onKeyDown={(event) => handleTabKeyDown(event, index)}
          >
            {item.label}
          </button>
        ))}
      </div>

      <div
        aria-labelledby={`product-tab-${activeItem.id}`}
        className={styles.panel}
        id="product-tour-panel"
        role="tabpanel"
      >
        <AnimatePresence initial={false} mode="wait">
          <m.div
            animate={{ opacity: 1, x: 0 }}
            className={styles.copy}
            exit={{ opacity: 0, x: reducedMotion ? 0 : -10 }}
            initial={{ opacity: 0, x: reducedMotion ? 0 : 10 }}
            key={activeItem.id}
            transition={reducedMotion ? { duration: 0 } : quickTransition}
          >
            <p>{activeItem.label}</p>
            <h3>{activeItem.title}</h3>
            <span>{activeItem.description}</span>
            <Link to="/signup">
              Explore with your corpus{" "}
              <ArrowRight aria-hidden="true" size={16} />
            </Link>
          </m.div>
        </AnimatePresence>

        <AnimatePresence initial={false} mode="wait">
          <m.figure
            animate={{ opacity: 1, scale: 1 }}
            className={styles.media}
            exit={{ opacity: 0, scale: reducedMotion ? 1 : 0.99 }}
            initial={{ opacity: 0, scale: reducedMotion ? 1 : 1.01 }}
            key={activeItem.image}
            transition={reducedMotion ? { duration: 0 } : motionTransition}
          >
            <img alt={activeItem.alt} loading="lazy" src={activeItem.image} />
          </m.figure>
        </AnimatePresence>
      </div>
    </m.section>
  );
}
