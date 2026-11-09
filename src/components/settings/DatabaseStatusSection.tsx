import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "@/lib/toast";
import { AlertTriangle, CheckCircle2, Loader2, RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  backupsApi,
  type DatabaseStatus,
  type DatabaseProbeState,
} from "@/lib/api/settings";
import { extractErrorMessage } from "@/utils/errorUtils";

function stateLabel(
  state: DatabaseProbeState,
  t: (key: string) => string,
): string {
  switch (state) {
    case "missing":
      return t("settings.databaseStatus.stateMissing");
    case "compatible":
      return t("settings.databaseStatus.stateCompatible");
    case "older":
      return t("settings.databaseStatus.stateOlder");
    case "tooNew":
      return t("settings.databaseStatus.stateTooNew");
    case "unusable":
      return t("settings.databaseStatus.stateUnusable");
    default:
      return state;
  }
}

function formatSyncTime(epochSeconds?: number | null): string {
  if (!epochSeconds) return "—";
  try {
    return new Date(epochSeconds * 1000).toLocaleString();
  } catch {
    return String(epochSeconds);
  }
}

/**
 * 显示当前使用的是主库还是 fork 私有备用库，以及单向同步状态。
 *
 * 正常情况下主库可用，这里只显示路径与版本；主库版本与当前应用不一致
 * （例如官方版升级过数据库）时会切到 mod_ 备用库，并提示原因。
 */
export function DatabaseStatusSection() {
  const { t } = useTranslation();
  const [status, setStatus] = useState<DatabaseStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [syncing, setSyncing] = useState(false);

  const refresh = useCallback(async () => {
    try {
      const next = await backupsApi.getDatabaseStatus();
      setStatus(next);
    } catch (error) {
      console.error("[DatabaseStatusSection] failed to load status", error);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const handleForceSync = useCallback(async () => {
    const confirmed = window.confirm(
      t("settings.databaseStatus.forceSyncConfirm"),
    );
    if (!confirmed) return;
    setSyncing(true);
    try {
      await backupsApi.forceSyncDatabase();
      toast.success(t("settings.databaseStatus.forceSyncSuccess"));
      await refresh();
    } catch (error) {
      toast.error(
        t("settings.databaseStatus.forceSyncFailed", {
          error: extractErrorMessage(error),
        }),
      );
    } finally {
      setSyncing(false);
    }
  }, [refresh, t]);

  if (loading) {
    return (
      <div className="flex items-center gap-2 text-sm text-fg-2">
        <Loader2 className="h-4 w-4 animate-spin" />
        {t("settings.databaseStatus.loading")}
      </div>
    );
  }

  if (!status) {
    return (
      <div className="text-sm text-fg-2">
        {t("settings.databaseStatus.unavailable")}
      </div>
    );
  }

  const fallbackActive = status.isFallback;

  return (
    <div className="space-y-4 text-sm">
      <div className="flex items-start gap-2">
        {fallbackActive ? (
          <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0 text-amber-500" />
        ) : (
          <CheckCircle2 className="mt-0.5 h-4 w-4 shrink-0 text-emerald-500" />
        )}
        <div className="min-w-0 space-y-1">
          <div className="font-medium">
            {fallbackActive
              ? t("settings.databaseStatus.modeFallback")
              : t("settings.databaseStatus.modePrimary")}
          </div>
          <div className="break-all font-mono text-xs text-fg-2">
            {status.activePath}
          </div>
          {fallbackActive && (
            <p className="text-xs text-fg-2">
              {t("settings.databaseStatus.fallbackExplanation")}
            </p>
          )}
        </div>
      </div>

      <div className="grid gap-2 sm:grid-cols-2">
        <ProbeRow
          label={t("settings.databaseStatus.primaryLabel")}
          path={status.primary.path}
          state={status.primary.state}
          version={status.primary.version}
          t={t}
        />
        <ProbeRow
          label={t("settings.databaseStatus.fallbackLabel")}
          path={status.fallback.path}
          state={status.fallback.state}
          version={status.fallback.version}
          t={t}
        />
      </div>

      <div className="flex flex-wrap items-center gap-3 text-xs text-fg-2">
        <span>
          {t("settings.databaseStatus.lastSync")}:{" "}
          {formatSyncTime(status.lastSyncAt)}
        </span>
        {status.fallbackDirty && (
          <span className="text-amber-500">
            {t("settings.databaseStatus.dirtyHint")}
          </span>
        )}
      </div>

      <Button
        type="button"
        variant="outline"
        size="sm"
        disabled={fallbackActive || syncing}
        onClick={() => void handleForceSync()}
      >
        {syncing ? (
          <Loader2 className="mr-2 h-4 w-4 animate-spin" />
        ) : (
          <RefreshCw className="mr-2 h-4 w-4" />
        )}
        {t("settings.databaseStatus.forceSync")}
      </Button>
    </div>
  );
}

function ProbeRow({
  label,
  path,
  state,
  version,
  t,
}: {
  label: string;
  path: string;
  state: DatabaseProbeState;
  version?: number | null;
  t: (key: string) => string;
}) {
  return (
    <div className="rounded-panel border border-border bg-surface p-3">
      <div className="flex items-center justify-between gap-2">
        <span className="font-medium">{label}</span>
        <span className="text-xs text-fg-2">
          {stateLabel(state, t)}
          {version != null ? ` · v${version}` : ""}
        </span>
      </div>
      <div className="mt-1 break-all font-mono text-xs text-fg-2">{path}</div>
    </div>
  );
}
