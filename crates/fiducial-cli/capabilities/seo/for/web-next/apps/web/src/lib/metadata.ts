import { domain } from "@/generated/brand";
import { seoFor } from "@/generated/seo";
import type { Metadata } from "next";

/**
 * A page's metadata, from the route table `fid derive` writes.
 *
 * `path` is the URL as a reader sees it: `/blog/hello` in the default locale,
 * `/sr/blog/hello` in another. Everything else — title, description, the
 * canonical URL, every language's alternate, the social image, an article's
 * date — is already in `generated/seo.ts`, derived from the same declarations
 * the sitemap is, so a page's tags and the sitemap cannot disagree.
 *
 * A path with no entry gets the site-wide metadata rather than an error: a
 * route can exist before it is declared, and a missing tag is not worth a 500.
 */
export function metadataForPath(path: string): Metadata {
  const route = seoFor(path);
  if (!route) return { metadataBase: new URL(`https://${domain}`) };
  const article = route.kind === "article";
  return {
    metadataBase: new URL(`https://${domain}`),
    title: route.title,
    description: route.description,
    alternates: { canonical: route.path, languages: { ...route.alternates } },
    openGraph: {
      type: article ? "article" : "website",
      url: route.path,
      title: route.title,
      description: route.description,
      ...(route.og ? { images: [{ url: route.og, width: 1200, height: 630 }] } : {}),
      ...(article && route.date ? { publishedTime: route.date } : {}),
    },
    twitter: {
      card: route.og ? "summary_large_image" : "summary",
      title: route.title,
      description: route.description,
    },
  };
}
