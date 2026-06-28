import { AlertTriangle, ArrowLeft, House, RefreshCw } from "lucide-react";
import { Component, type ErrorInfo, type ReactNode } from "react";
import { Link, useLocation, useNavigate } from "react-router-dom";

import styles from "./RouteErrorBoundary.module.css";

interface BoundaryProps {
  children: ReactNode;
  resetKey: string;
  onBack: () => void;
}

interface BoundaryState {
  error: Error | null;
}

class RouteErrorBoundaryInner extends Component<BoundaryProps, BoundaryState> {
  state: BoundaryState = { error: null };

  static getDerivedStateFromError(error: Error): BoundaryState {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("Workbench route failed", error, info.componentStack);
  }

  componentDidUpdate(previousProps: BoundaryProps) {
    if (this.state.error && previousProps.resetKey !== this.props.resetKey) {
      this.setState({ error: null });
    }
  }

  render() {
    if (!this.state.error) {
      return this.props.children;
    }

    return (
      <section className={styles.state} role="alert">
        <span className={styles.icon}>
          <AlertTriangle aria-hidden="true" size={20} />
        </span>
        <h1>This view could not be opened</h1>
        <p>
          Your data is still safe. Retry this view, return to the previous page,
          or continue from Home.
        </p>
        <div className={styles.actions}>
          <button type="button" onClick={() => this.setState({ error: null })}>
            <RefreshCw aria-hidden="true" size={16} /> Retry
          </button>
          <button type="button" onClick={this.props.onBack}>
            <ArrowLeft aria-hidden="true" size={16} /> Back
          </button>
          <Link to="/app">
            <House aria-hidden="true" size={16} /> Home
          </Link>
        </div>
      </section>
    );
  }
}

export function RouteErrorBoundary({ children }: { children: ReactNode }) {
  const location = useLocation();
  const navigate = useNavigate();

  return (
    <RouteErrorBoundaryInner
      resetKey={`${location.pathname}${location.search}`}
      onBack={() => navigate(-1)}
    >
      {children}
    </RouteErrorBoundaryInner>
  );
}
