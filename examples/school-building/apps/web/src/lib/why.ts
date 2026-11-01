import { type Locale, messages } from "../generated/messages"
import type { Outcome } from "../generated/wire/Outcome"
import type { Reason } from "../generated/wire/Reason"

export const why = (reason: Reason, locale: Locale): string => messages[locale][`reason.${reason}`]

export const outcome = (o: Outcome, locale: Locale): string =>
  o === "executed" ? messages[locale]["outcome.executed"] : why(o.refused, locale)
