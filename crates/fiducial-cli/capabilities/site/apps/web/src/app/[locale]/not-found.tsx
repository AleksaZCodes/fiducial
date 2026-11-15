import { defaultLocale } from "@/generated/messages";
import { localePath } from "@/lib/routes";
import { t } from "@/lib/t";
import Link from "next/link";

// A not-found page cannot read the route's params, so it speaks the default
// locale; the header and footer around it are still the reader's.
export default function NotFound() {
  return (
    <div className="flex flex-col items-start gap-4">
      <h1 className="type-h1">{t(defaultLocale, "site.notFound")}</h1>
      <Link className="type-body underline" href={localePath(defaultLocale, "/")}>
        {t(defaultLocale, "site.backHome")}
      </Link>
    </div>
  );
}
