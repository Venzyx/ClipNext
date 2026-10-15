import { memo, useEffect, useState } from "react";
import { api } from "../services/api";
import type { Clip } from "../types";
import { useI18n } from "../i18n";
import { formatTime } from "../services/time";
import { ImageIcon, LinkIcon, PinIcon, TextIcon, TrashIcon } from "./Icons";

const thumbCache = new Map<number, string>();

function useThumbnail(id: number, enabled: boolean): string | null {
  const [src, setSrc] = useState<string | null>(thumbCache.get(id) ?? null);
  useEffect(() => {
    if (!enabled || thumbCache.has(id)) return;
    let cancelled = false;
    api
      .getThumbnail(id)
      .then((url) => {
        thumbCache.set(id, url);
        if (!cancelled) setSrc(url);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [id, enabled]);
  return src;
}

interface Props {
  clip: Clip;
  index: number;
  selected: boolean;
  onSelect: (index: number) => void;
  onActivate: (clip: Clip) => void;
  onTogglePin: (clip: Clip) => void;
  onDelete: (clip: Clip) => void;
}

function ClipItemView({
  clip,
  index,
  selected,
  onSelect,
  onActivate,
  onTogglePin,
  onDelete,
}: Props) {
  const { t, lang } = useI18n();
  const isImage = clip.kind === "image";
  const isLink = clip.kind === "link";
  const thumb = useThumbnail(clip.id, isImage);

  return (
    <li
      role="option"
      aria-selected={selected}
      data-index={index}
      onMouseMove={() => !selected && onSelect(index)}
      onClick={() => onActivate(clip)}
      className={`group relative flex cursor-default items-center gap-3 rounded-2xl px-3 py-2.5 transition-colors duration-150 ${
        selected ? "bg-surface shadow-soft" : ""
      }`}
    >
      <div
        className={`flex h-11 w-11 shrink-0 items-center justify-center overflow-hidden rounded-xl ${
          isImage
            ? "bg-fill"
            : isLink
              ? "bg-accent/12 text-accent"
              : "bg-fill text-muted"
        }`}
      >
        {isImage && thumb ? (
          <img
            src={thumb}
            alt=""
            className="h-full w-full object-cover"
            draggable={false}
          />
        ) : isLink ? (
          <LinkIcon />
        ) : isImage ? (
          <ImageIcon />
        ) : (
          <TextIcon />
        )}
      </div>

      <div className="min-w-0 flex-1">
        <p
          className={`line-clamp-2 break-words text-[13.5px] leading-[1.35] ${
            isLink ? "text-accent" : "text-ink"
          }`}
        >
          {isImage ? t("imageLabel") : clip.text}
        </p>
        <p className="mt-0.5 flex items-center gap-1 text-xs text-muted">
          {clip.pinned && <PinIcon filled width={11} height={11} className="text-accent" />}
          {formatTime(clip.createdAt, lang, t("justNow"))}
        </p>
      </div>

      <div
        className={`flex shrink-0 items-center gap-0.5 transition-opacity duration-150 ${
          selected ? "opacity-100" : "opacity-0"
        }`}
      >
        <button
          type="button"
          title={clip.pinned ? t("unpin") : t("pin")}
          aria-label={clip.pinned ? t("unpin") : t("pin")}
          onClick={(e) => {
            e.stopPropagation();
            onTogglePin(clip);
          }}
          className={`rounded-lg p-1.5 transition-colors hover:bg-fill ${
            clip.pinned ? "text-accent" : "text-muted"
          }`}
        >
          <PinIcon filled={clip.pinned} width={16} height={16} />
        </button>
        <button
          type="button"
          title={t("delete")}
          aria-label={t("delete")}
          onClick={(e) => {
            e.stopPropagation();
            onDelete(clip);
          }}
          className="rounded-lg p-1.5 text-muted transition-colors hover:bg-fill hover:text-danger"
        >
          <TrashIcon width={16} height={16} />
        </button>
      </div>
    </li>
  );
}

export const ClipItem = memo(ClipItemView);
