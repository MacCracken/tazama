import { useProjectStore } from "./stores/projectStore";
import { useUIStore } from "./stores/uiStore";
import { AppShell } from "./components/layout/AppShell";
import { WelcomeScreen } from "./components/project/WelcomeScreen";
import { NewProjectDialog } from "./components/project/NewProjectDialog";
import { ExportDialog } from "./components/export/ExportDialog";
import { useKeyboardShortcuts } from "./hooks/useKeyboardShortcuts";
import { ErrorBoundary } from "./components/shared/ErrorBoundary";
import { Toast } from "./components/shared/Toast";

export default function App() {
  useKeyboardShortcuts();
  const project = useProjectStore((s) => s.project);
  const showNewProjectDialog = useUIStore((s) => s.showNewProjectDialog);
  const showExportDialog = useUIStore((s) => s.showExportDialog);

  return (
    <ErrorBoundary>
      <div className="app">
        {project ? <AppShell /> : <WelcomeScreen />}
        {showNewProjectDialog && <NewProjectDialog />}
        {showExportDialog && <ExportDialog />}
        <Toast />
      </div>
    </ErrorBoundary>
  );
}
