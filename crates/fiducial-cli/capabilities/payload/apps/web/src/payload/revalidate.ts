import type { CollectionAfterChangeHook, GlobalAfterChangeHook } from "payload";

export const revalidatePage: CollectionAfterChangeHook = async ({ doc }) => {
  const { revalidatePath, revalidateTag } = await import("next/cache");
  const slug = doc.slug as string | undefined;
  if (slug) {
    revalidatePath(`/${slug}`);
    revalidateTag(`page:${slug}`);
  }
  revalidatePath("/");
  revalidateTag("pages");
};

export const revalidateSiteSettings: GlobalAfterChangeHook = async () => {
  const { revalidatePath, revalidateTag } = await import("next/cache");
  revalidatePath("/", "layout");
  revalidateTag("site-settings");
};
