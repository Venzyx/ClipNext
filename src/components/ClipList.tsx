import { useEffect, useRef } from "react";
import type { Clip } from "../types";
import { useI18n } from "../i18n";
import { ClipItem } from "./ClipItem";
import { ClipboardIcon, SearchIcon } from "./Icons";

interface Props {
  clips: Clip[];
  selectedIndex: number;
  searching: boolean;
  multi: number[];
  onSelect: (index: number) => void;
  onActivate: (clip: Clip) => void;
  onToggleMulti: (clip: Clip) => void;
  onTogglePin: (clip: Clip) => void;
  onDelete: (clip: Clip) => void;
}

export function ClipList({
  clips,
  selectedIndex,
  searching,
  multi,
  onSelect,
  onActivate,
  onToggleMulti,
  onTogglePin,
  onDelete,
}: Props) {
  const { t } = useI18n();
  const listRef = useRef<HTMLUListElement>(null);

  useEffect(() => {
    listRef.current
      ?.querySelector<HTMLElement>(`[data-index="${selectedIndex}"]`)
      ?.scrollIntoView({ block: "nearest" });
  }, [selectedIndex, clips]);

  if (clips.length === 0) {
    const Icon = searching ? SearchIcon : ClipboardIcon;
    return (
      <div className="flex flex-1 flex-col items-center justify-center gap-2 px-10 text-center animate-fade-in">
        <div className="mb-1 flex h-14 w-14 items-center justify-center rounded-2xl bg-fill text-muted">
          <Icon width={26} height={26} />
        </div>
        <p className="text-[15px] font-medium text-ink">
          {searching ? t("noResultsTitle") : t("emptyTitle")}
        </p>
        <p className="text-[13px] leading-snug text-muted">
          {searching ? t("noResultsBody") : t("emptyBody")}
        </p>
      </div>
    );
  }

  return (
    <ul
      ref={listRef}
      role="listbox"
      className="scroll-area flex-1 space-y-0.5 overflow-y-auto px-2 pb-2"
    >
      {clips.map((clip, index) => (
        <ClipItem
          key={clip.id}
          clip={clip}
          index={index}
          selected={index === selectedIndex}
          multiIndex={multi.includes(clip.id) ? multi.indexOf(clip.id) + 1 : undefined}
          multiActive={multi.length > 0}
          onSelect={onSelect}
          onActivate={onActivate}
          onToggleMulti={onToggleMulti}
          onTogglePin={onTogglePin}
          onDelete={onDelete}
        />
      ))}
    </ul>
  );
}
