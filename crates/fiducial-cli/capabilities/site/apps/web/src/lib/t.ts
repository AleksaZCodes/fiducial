import { type Locale, type MessageKey, messages } from "@/generated/messages";

/** One message, in one locale, with `{name}` placeholders filled in. */
export function t(
  locale: Locale,
  key: MessageKey,
  vars: Record<string, string | number> = {},
): string {
  return messages[locale][key].replace(/\{(\w+)\}/g, (_, name: string) => String(vars[name] ?? ""));
}
