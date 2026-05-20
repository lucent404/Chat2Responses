import { useEffect, useMemo, useState } from "react";
import { CircleDot, Command, LogOut, RefreshCcw } from "lucide-react";
import { NavLink, Navigate, useParams } from "react-router-dom";
import { getCodexCatalogStatus, getSettings, logoutAdmin, listApiKeys, listAvailableModels, listModelRoutes, listRequestLogs, listUpstreams, updateSettings } from "../api/admin";
import { BrandBlock } from "../components/common/BrandBlock";
import { PanelHeader } from "../components/common/PanelHeader";
import { Button } from "../components/ui/button";
import { ApiKeysPanel } from "../features/api-keys/ApiKeysPanel";
import { LogsPanel } from "../features/logs/LogsPanel";
import { ModelsPanel } from "../features/models/ModelsPanel";
import { OverviewPanel } from "../features/overview/OverviewPanel";
import { SettingsPanel } from "../features/settings/SettingsPanel";
import { TutorialPanel } from "../features/tutorial/TutorialPanel";
import { UpstreamsPanel } from "../features/upstreams/UpstreamsPanel";
import type { ApiKey, AppSettings, AvailableModel, CodexCatalogStatus, ModelRoute, PageState, RequestLog, ToastState, Upstream } from "../types/admin";
import { navItems, type Tab } from "./navigation";

type AdminShellProps = {
  user: string;
  onLogout: () => Promise<void>;
  setToast: (toast: ToastState) => void;
};

const initialPageState: PageState = { page: 1, pageSize: 20, q: "", total: 0, totalPages: 0 };
const defaultSettings: AppSettings = {
  request_logging_enabled: false,
  upstream_timeout_seconds: 0,
  log_error_max_chars: 500
};

export function AdminShell({ user, onLogout, setToast }: AdminShellProps) {
  const { section } = useParams();
  const tab = useMemo(() => {
    if (!section) return "overview";
    return navItems.some((item) => item.id === section) ? (section as Tab) : null;
  }, [section]);
  const [upstreams, setUpstreams] = useState<Upstream[]>([]);
  const [upstreamOptions, setUpstreamOptions] = useState<Upstream[]>([]);
  const [models, setModels] = useState<ModelRoute[]>([]);
  const [availableModels, setAvailableModels] = useState<AvailableModel[]>([]);
  const [keys, setKeys] = useState<ApiKey[]>([]);
  const [logs, setLogs] = useState<RequestLog[]>([]);
  const [settings, setSettings] = useState<AppSettings>(defaultSettings);
  const [catalogStatus, setCatalogStatus] = useState<CodexCatalogStatus | null>(null);
  const [overviewTotals, setOverviewTotals] = useState({
    upstreams: 0,
    models: 0,
    keys: 0,
    logs: 0
  });
  const [upstreamPage, setUpstreamPage] = useState<PageState>(initialPageState);
  const [modelPage, setModelPage] = useState<PageState>(initialPageState);
  const [keyPage, setKeyPage] = useState<PageState>(initialPageState);
  const [logPage, setLogPage] = useState<PageState>(initialPageState);
  const [refreshing, setRefreshing] = useState(true);

  const loadUpstreams = async () => {
    const response = await listUpstreams({ page: upstreamPage.page, pageSize: upstreamPage.pageSize, q: upstreamPage.q });
    setUpstreams(response.items);
    if (!upstreamPage.q) setUpstreamOptions(response.items);
    setUpstreamPage((current) => pageFromResponse(response, current.q));
  };

  const loadUpstreamOptions = async () => {
    const response = await listUpstreams({ page: 1, pageSize: 100, q: "" });
    setUpstreamOptions(response.items);
  };

  const loadModels = async () => {
    const [routeResponse] = await Promise.all([
      listModelRoutes({ page: modelPage.page, pageSize: modelPage.pageSize, q: modelPage.q }),
      loadUpstreamOptions()
    ]);
    setModels(routeResponse.items);
    setModelPage((current) => pageFromResponse(routeResponse, current.q));
  };

  const loadKeys = async () => {
    const response = await listApiKeys({ page: keyPage.page, pageSize: keyPage.pageSize, q: keyPage.q });
    setKeys(response.items);
    setKeyPage((current) => pageFromResponse(response, current.q));
  };

  const loadLogs = async () => {
    const [response, nextSettings] = await Promise.all([
      listRequestLogs({ page: logPage.page, pageSize: logPage.pageSize, q: logPage.q }),
      getSettings()
    ]);
    setLogs(response.items);
    setSettings(nextSettings);
    setLogPage((current) => pageFromResponse(response, current.q));
  };

  const loadSettings = async () => {
    const nextSettings = await getSettings();
    setSettings(nextSettings);
  };

  const loadTutorial = async () => {
    const nextStatus = await getCodexCatalogStatus();
    setCatalogStatus(nextStatus);
  };

  const loadOverview = async () => {
    const [nextUpstreams, nextModels, nextAvailableModels, nextKeys, nextLogs, nextSettings] = await Promise.all([
      listUpstreams({ page: 1, pageSize: 20, q: "" }),
      listModelRoutes({ page: 1, pageSize: 20, q: "" }),
      listAvailableModels(),
      listApiKeys({ page: 1, pageSize: 20, q: "" }),
      listRequestLogs({ page: 1, pageSize: 20, q: "" }),
      getSettings()
    ]);
    setUpstreams(nextUpstreams.items);
    setUpstreamOptions(nextUpstreams.items);
    setModels(nextModels.items);
    setAvailableModels(nextAvailableModels);
    setKeys(nextKeys.items);
    setLogs(nextLogs.items);
    setSettings(nextSettings);
    setOverviewTotals({
      upstreams: nextUpstreams.total,
      models: nextModels.total,
      keys: nextKeys.total,
      logs: nextLogs.total
    });
  };

  const refresh = async () => {
    if (!tab) return;
    setRefreshing(true);
    try {
      if (tab === "overview") await loadOverview();
      if (tab === "upstreams") await loadUpstreams();
      if (tab === "models") await loadModels();
      if (tab === "keys") await loadKeys();
      if (tab === "logs") await loadLogs();
      if (tab === "settings") await loadSettings();
      if (tab === "tutorial") await loadTutorial();
    } finally {
      setRefreshing(false);
    }
  };

  const saveSettings = async (nextSettings: AppSettings) => {
    const saved = await updateSettings(nextSettings);
    setSettings(saved);
  };

  useEffect(() => {
    refresh()
      .catch((error) => setToast({ type: "error", message: error.message }))
      .finally(() => setRefreshing(false));
  }, [tab, upstreamPage.page, upstreamPage.pageSize, upstreamPage.q, modelPage.page, modelPage.pageSize, modelPage.q, keyPage.page, keyPage.pageSize, keyPage.q, logPage.page, logPage.pageSize, logPage.q]);

  const logout = async () => {
    await logoutAdmin();
    await onLogout();
  };

  if (!section) return <Navigate to="/admin/overview" replace />;
  if (!tab) return <Navigate to="/admin/overview" replace />;

  return (
    <main className="relative h-screen overflow-hidden bg-background">
      <div className="ambient-grid pointer-events-none absolute inset-0" />
      <div className="noise-overlay pointer-events-none absolute inset-0" />
      <div className="relative grid h-screen grid-cols-[292px_minmax(0,1fr)] gap-5 overflow-hidden p-5 max-[1100px]:grid-cols-1 max-[1100px]:grid-rows-[auto_minmax(0,1fr)] max-[1100px]:gap-4 max-[760px]:p-3">
      <aside className="glass-rail flex h-full min-h-0 flex-col gap-5 rounded-[28px] p-5 max-[1100px]:h-auto max-[1100px]:rounded-3xl max-[1100px]:p-3">
        <div className="flex items-center justify-between gap-3">
          <BrandBlock subtitle={user} compact />
          <Button variant="ghost" size="icon" className="hidden max-[1100px]:inline-flex" onClick={logout} aria-label="退出">
            <LogOut size={18} />
          </Button>
        </div>
        <div className="flex items-center gap-2 rounded-2xl border border-slate-300/60 bg-white/70 px-3 py-2 font-mono text-xs text-slate-600 shadow-sm">
          <Command size={14} />
          <span>relay://127.0.0.1</span>
        </div>
        <nav className="grid gap-1.5 max-[1100px]:flex max-[1100px]:overflow-x-auto">
          {navItems.map((item) => (
            <NavButton key={item.id} to={item.path} icon={item.icon} label={item.label} />
          ))}
        </nav>
        <div className="mt-auto grid gap-3 rounded-2xl border border-slate-300/60 bg-white/72 p-3 text-xs text-muted-foreground shadow-sm backdrop-blur-xl max-[1100px]:hidden">
          <div className="flex items-center justify-between">
            <span className="font-semibold text-foreground">Console session</span>
            <span className="relative flex h-2.5 w-2.5">
              <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-emerald-400 opacity-60" />
              <span className="relative inline-flex h-2.5 w-2.5 rounded-full bg-emerald-500" />
            </span>
          </div>
          <span className="font-mono">Local relay admin for `127.0.0.1`</span>
        </div>
        <Button variant="ghost" className="justify-start max-[1100px]:hidden" onClick={logout}>
          <LogOut size={18} />
          退出登录
        </Button>
      </aside>
      <section className="min-h-0 min-w-0 overflow-y-auto py-1 pr-1 max-[1100px]:p-0">
        <PanelHeader tab={tab} refresh={refresh} refreshing={refreshing} setToast={setToast} />
        {refreshing ? (
          <LoadingPanels />
        ) : (
          <>
            {tab === "overview" && (
              <OverviewPanel
                upstreams={upstreams}
                upstreamTotal={overviewTotals.upstreams}
                models={models}
                modelTotal={overviewTotals.models}
                availableModels={availableModels}
                keys={keys}
                keyTotal={overviewTotals.keys}
                logs={logs}
                logTotal={overviewTotals.logs}
                settings={settings}
              />
            )}
            {tab === "upstreams" && <UpstreamsPanel rows={upstreams} pageState={upstreamPage} setPageState={setUpstreamPage} refresh={refresh} setToast={setToast} />}
            {tab === "models" && <ModelsPanel rows={models} pageState={modelPage} setPageState={setModelPage} upstreams={upstreamOptions} refresh={refresh} setToast={setToast} />}
            {tab === "keys" && <ApiKeysPanel rows={keys} pageState={keyPage} setPageState={setKeyPage} refresh={refresh} setToast={setToast} />}
            {tab === "logs" && <LogsPanel rows={logs} pageState={logPage} setPageState={setLogPage} settings={settings} />}
            {tab === "settings" && <SettingsPanel settings={settings} refresh={refresh} saveSettings={saveSettings} setToast={setToast} />}
            {tab === "tutorial" && <TutorialPanel catalogStatus={catalogStatus} refresh={refresh} setToast={setToast} />}
          </>
        )}
      </section>
      </div>
    </main>
  );
}

function pageFromResponse<T>(response: { page: number; page_size: number; total: number; total_pages: number }, q: string): PageState {
  return {
    page: response.total_pages > 0 ? Math.min(response.page, response.total_pages) : response.page,
    pageSize: response.page_size,
    q,
    total: response.total,
    totalPages: response.total_pages
  };
}

function NavButton({ to, icon, label }: { to: string; icon: React.ReactNode; label: string }) {
  return (
    <NavLink to={to} className="max-[1100px]:shrink-0">
      {({ isActive }) => (
        <Button
          asChild
          variant={isActive ? "secondary" : "ghost"}
          className={`relative w-full justify-start overflow-hidden max-[1100px]:w-auto ${
            isActive
              ? "border border-blue-300/70 bg-blue-600 text-white shadow-glow hover:bg-blue-600 hover:text-white"
              : "text-slate-600 hover:bg-white/75 hover:text-slate-950"
          }`}
        >
          <span>
            {isActive ? <CircleDot size={10} className="text-cyan-200" /> : null}
            {icon}
            {label}
          </span>
        </Button>
      )}
    </NavLink>
  );
}

function LoadingPanels() {
  return (
    <div className="grid gap-4">
      <div className="h-40 animate-pulse rounded-lg border border-border bg-card" />
      <div className="grid grid-cols-3 gap-4 max-[760px]:grid-cols-1">
        <div className="h-28 animate-pulse rounded-lg border border-border bg-card" />
        <div className="h-28 animate-pulse rounded-lg border border-border bg-card" />
        <div className="h-28 animate-pulse rounded-lg border border-border bg-card" />
      </div>
    </div>
  );
}
