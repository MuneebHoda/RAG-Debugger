import { type KeyboardEvent, useRef } from "react";

type TabId = string;

interface RovingTabsOptions<Id extends TabId> {
  ids: readonly Id[];
  onSelect: (id: Id) => void;
}

export function useRovingTabs<Id extends TabId>({
  ids,
  onSelect,
}: RovingTabsOptions<Id>) {
  const tabRefs = useRef<Array<HTMLButtonElement | null>>([]);

  function registerTab(index: number) {
    return (element: HTMLButtonElement | null) => {
      tabRefs.current[index] = element;
    };
  }

  function handleTabKeyDown(
    event: KeyboardEvent<HTMLButtonElement>,
    index: number,
  ) {
    if (event.key !== "ArrowRight" && event.key !== "ArrowLeft") return;

    event.preventDefault();
    const direction = event.key === "ArrowRight" ? 1 : -1;
    const nextIndex = (index + direction + ids.length) % ids.length;
    const nextId = ids[nextIndex];

    onSelect(nextId);
    tabRefs.current[nextIndex]?.focus();
  }

  return { handleTabKeyDown, registerTab };
}
