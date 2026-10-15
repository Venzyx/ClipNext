import { useState } from "react";
import { useI18n } from "../i18n";

interface Props {
  count: number;
  pinnedCount: number;
  onCancel: () => void;
  onConfirm: (includePinned: boolean) => void;
}

export function ConfirmDialog({ count, pinnedCount, onCancel, onConfirm }: Props) {
  const { t } = useI18n();
  const [includePinned, setIncludePinned] = useState(false);
  const removing = includePinned ? count : count - pinnedCount;

  return (
    <div
      className="absolute inset-0 z-30 flex items-center justify-center bg-black/30 px-8 animate-fade-in"
      onClick={onCancel}
    >
      <div
        role="alertdialog"
        aria-modal="true"
        onClick={(e) => e.stopPropagation()}
        className="w-full rounded-3xl bg-surface p-5 shadow-pop animate-pop"
      >
        <h2 className="text-[16px] font-semibold text-ink">{t("clearTitle")}</h2>
        <p className="mt-1.5 text-[13px] leading-snug text-muted">
          {t("clearBody", { n: removing })}
        </p>
        {pinnedCount > 0 && (
          <label className="mt-4 flex cursor-default items-center gap-2 text-[13px] text-ink">
            <input
              type="checkbox"
              checked={includePinned}
              onChange={(e) => setIncludePinned(e.target.checked)}
              className="h-4 w-4 accent-[var(--accent)]"
            />
            {t("clearPinnedToo")}
          </label>
        )}
        <div className="mt-5 flex gap-2">
          <button
            type="button"
            autoFocus
            onClick={onCancel}
            className="flex-1 rounded-xl bg-fill py-2 text-[14px] font-medium text-ink transition-colors hover:brightness-95"
          >
            {t("cancel")}
          </button>
          <button
            type="button"
            onClick={() => onConfirm(includePinned)}
            className="flex-1 rounded-xl bg-danger py-2 text-[14px] font-medium text-white transition-colors hover:brightness-95"
          >
            {t("clear")}
          </button>
        </div>
      </div>
    </div>
  );
}
