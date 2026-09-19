import { mediaUrl } from "@/lib/media";

/**
 * One image with its caption and a download button.
 *
 * Every image on a document page goes through this: press gallery items, story
 * and post covers, and images inside a post's body. One component because a
 * press asset is used by someone else, and a journalist lifting an image needs
 * the same three things wherever they find it — the picture, what it shows,
 * and a way to save the original rather than a screenshot of it.
 *
 * The download goes through `?download`, which makes the media route send
 * `Content-Disposition: attachment`. A bare `download` attribute is ignored by
 * some browsers for anything they can display, which would open the image in
 * a tab instead of saving it.
 */
export function MediaFigure({
  mediaKey,
  alt,
  caption,
  downloadLabel,
  aspect = "aspect-[3/2]",
  fit = "cover",
  priority = false,
}: {
  mediaKey: string;
  alt: string;
  caption?: string;
  /** Omit to show the image without a download button (e.g. a card thumbnail). */
  downloadLabel?: string;
  aspect?: string;
  /**
   * `contain` for an asset whose whole frame matters — a logo, an icon, a
   * diagram — where cropping would misrepresent the file being downloaded.
   */
  fit?: "cover" | "contain";
  priority?: boolean;
}) {
  const src = mediaUrl(mediaKey);
  return (
    <figure className="flex flex-col gap-3">
      <div className={`cham overflow-hidden p-0 ${aspect}`}>
        {/* eslint-disable-next-line @next/next/no-img-element */}
        <img
          src={src}
          alt={alt}
          loading={priority ? "eager" : "lazy"}
          decoding="async"
          className={`relative z-0 h-full w-full ${fit === "contain" ? "object-contain" : "object-cover"}`}
        />
      </div>
      {caption || downloadLabel ? (
        <figcaption className="flex items-start justify-between gap-4">
          {caption ? <span className="type-small text-muted-foreground">{caption}</span> : <span />}
          {downloadLabel ? (
            <a
              href={`${src}?download`}
              className="cham-sm cham-outline type-small shrink-0 px-3 py-1.5 font-medium"
            >
              {downloadLabel}
            </a>
          ) : null}
        </figcaption>
      ) : null}
    </figure>
  );
}
