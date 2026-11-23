"use client";

import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { useCallback } from "react";

interface VariantOption {
  id: string;
  title: string;
  dimensions?: string | null;
  material?: string | null;
  color?: string | null;
  price?: { amountMinor?: number; currency?: string } | null;
}

export default function VariantPicker({
  variants,
}: {
  variants: VariantOption[];
}) {
  const router = useRouter();
  const pathname = usePathname();
  const searchParams = useSearchParams();
  const selectedId = searchParams.get("variant") ?? (variants[0]?.id ?? "");

  const select = useCallback(
    (id: string) => {
      const p = new URLSearchParams(searchParams.toString());
      p.set("variant", id);
      router.replace(`${pathname}?${p.toString()}`, { scroll: false });
      try {
        fetch("/api/e", {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({ event: "add_to_cart", path: pathname }),
        });
      } catch {}
    },
    [router, pathname, searchParams],
  );

  if (!variants.length) return null;

  return (
    <fieldset className="mt-8">
      <legend className="text-sm font-semibold">Select variant</legend>
      <div className="mt-3 flex flex-col gap-2">
        {variants.map((v) => (
          <label
            key={v.id}
            className={`flex cursor-pointer items-center gap-3 rounded-md border p-4 transition-colors ${
              selectedId === v.id
                ? "border-primary bg-accent"
                : "border-border hover:bg-muted/50"
            }`}
          >
            <input
              type="radio"
              name="variant"
              value={v.id}
              checked={selectedId === v.id}
              onChange={() => select(v.id)}
              className="sr-only"
            />
            <span className="flex-1">
              <span className="block font-medium">{v.title}</span>
              {(v.dimensions ?? v.material ?? v.color) && (
                <span className="mt-1 block text-sm text-muted-foreground">
                  {[v.dimensions, v.material, v.color].filter(Boolean).join(" · ")}
                </span>
              )}
            </span>
            {v.price?.amountMinor != null && (
              <span className="text-sm font-semibold">
                {(v.price.amountMinor / 100).toLocaleString()} {v.price.currency}
              </span>
            )}
          </label>
        ))}
      </div>
    </fieldset>
  );
}
