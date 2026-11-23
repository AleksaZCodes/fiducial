import type { CollectionAfterChangeHook, GlobalAfterChangeHook } from "payload";

export const revalidatePage: CollectionAfterChangeHook = async ({ doc }) => {
  const { revalidatePath, revalidateTag } = await import("next/cache");
  const slug = doc.slug as string | undefined;
  if (slug) {
    revalidatePath(`/${slug}`, "page");
    revalidateTag(`page:${slug}`, { expire: 0 });
  }
  revalidatePath("/", "page");
  revalidateTag("pages", { expire: 0 });
};

export const revalidateSiteSettings: GlobalAfterChangeHook = async () => {
  const { revalidatePath, revalidateTag } = await import("next/cache");
  revalidatePath("/", "layout");
  revalidateTag("site-settings", { expire: 0 });
};
