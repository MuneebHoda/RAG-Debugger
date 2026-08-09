import {
  Database,
  FileBarChart,
  FlaskConical,
  GitBranch,
  Home,
  KeyRound,
  Search,
  Settings,
  type LucideIcon,
} from "lucide-react";

export type WorkbenchNavItemId =
  | "home"
  | "corpus"
  | "retrieval"
  | "traces"
  | "eval_lab"
  | "ci_runs"
  | "reports"
  | "settings";

export interface WorkbenchNavItem {
  id: WorkbenchNavItemId;
  label: string;
  to: string;
  icon: LucideIcon;
}

export interface WorkbenchNavGroup {
  label: string;
  items: WorkbenchNavItem[];
}

export interface WorkbenchBreadcrumb {
  label: string;
  to?: string;
}

export const WORKBENCH_NAV_GROUPS: WorkbenchNavGroup[] = [
  {
    label: "Setup",
    items: [
      { id: "home", to: "/app", label: "Home", icon: Home },
      { id: "corpus", to: "/app/sources", label: "Corpus", icon: Database },
    ],
  },
  {
    label: "Debug",
    items: [
      {
        id: "retrieval",
        to: "/app/retrieval",
        label: "Retrieval",
        icon: Search,
      },
      {
        id: "traces",
        to: "/app/traces",
        label: "Trace Debugger",
        icon: GitBranch,
      },
    ],
  },
  {
    label: "Quality",
    items: [
      {
        id: "eval_lab",
        to: "/app/evals",
        label: "Eval Lab",
        icon: FlaskConical,
      },
      {
        id: "ci_runs",
        to: "/app/evals?view=ci-runs",
        label: "CI Runs",
        icon: KeyRound,
      },
    ],
  },
  {
    label: "Share",
    items: [
      {
        id: "reports",
        to: "/app/reports",
        label: "Audit Reports",
        icon: FileBarChart,
      },
    ],
  },
  {
    label: "Admin",
    items: [
      {
        id: "settings",
        to: "/app/settings",
        label: "Settings",
        icon: Settings,
      },
    ],
  },
];

const workflowIds: WorkbenchNavItemId[] = [
  "corpus",
  "retrieval",
  "traces",
  "eval_lab",
  "ci_runs",
  "reports",
];

const navItems = WORKBENCH_NAV_GROUPS.flatMap((group) => group.items);

export const WORKBENCH_FLOW_LABELS = workflowIds.map(
  (id) => navItems.find((item) => item.id === id)?.label ?? id,
);

export function isWorkbenchNavItemActive(
  itemId: WorkbenchNavItemId,
  pathname: string,
  search: string,
): boolean {
  const view = new URLSearchParams(search).get("view");

  switch (itemId) {
    case "home":
      return pathname === "/app" || pathname === "/app/";
    case "corpus":
      return pathname.startsWith("/app/sources");
    case "retrieval":
      return pathname.startsWith("/app/retrieval");
    case "traces":
      return pathname.startsWith("/app/traces");
    case "eval_lab":
      return (
        pathname.startsWith("/app/evals") &&
        !pathname.startsWith("/app/evals/ci-runs/") &&
        view !== "ci-runs"
      );
    case "ci_runs":
      return (
        (pathname === "/app/evals" && view === "ci-runs") ||
        pathname.startsWith("/app/evals/ci-runs/")
      );
    case "reports":
      return pathname.startsWith("/app/reports");
    case "settings":
      return pathname.startsWith("/app/settings");
  }
}

export function resolveWorkbenchBreadcrumbs(
  pathname: string,
  search: string,
): WorkbenchBreadcrumb[] {
  const home = { label: "Home", to: "/app" };
  const segments = pathname.split("/").filter(Boolean);
  const area = segments[1];
  const detailId = segments[2];

  if (!area) return [{ label: "Home" }];

  if (area === "sources") {
    return detailId
      ? [home, { label: "Corpus", to: "/app/sources" }, { label: "Document" }]
      : [home, { label: "Corpus" }];
  }
  if (area === "retrieval") return [home, { label: "Retrieval" }];
  if (area === "traces") {
    return detailId
      ? [
          home,
          { label: "Trace Debugger", to: "/app/traces" },
          { label: "Run detail" },
        ]
      : [home, { label: "Trace Debugger" }];
  }
  if (area === "evals") {
    if (detailId === "ci-runs") {
      return [
        home,
        { label: "Eval Lab", to: "/app/evals" },
        { label: "CI Runs", to: "/app/evals?view=ci-runs" },
        { label: "Run detail" },
      ];
    }
    if (new URLSearchParams(search).get("view") === "ci-runs") {
      return [
        home,
        { label: "Eval Lab", to: "/app/evals" },
        { label: "CI Runs" },
      ];
    }
    if (detailId === "datasets") {
      return [
        home,
        { label: "Eval Lab", to: "/app/evals" },
        { label: "Dataset" },
      ];
    }
    if (detailId === "experiments") {
      return [
        home,
        { label: "Eval Lab", to: "/app/evals" },
        { label: "Experiment" },
      ];
    }
    return [home, { label: "Eval Lab" }];
  }
  if (area === "reports") {
    return detailId
      ? [
          home,
          { label: "Audit Reports", to: "/app/reports" },
          { label: "Report detail" },
        ]
      : [home, { label: "Audit Reports" }];
  }
  if (area === "settings") return [home, { label: "Settings" }];

  return [home, { label: "Workspace" }];
}
