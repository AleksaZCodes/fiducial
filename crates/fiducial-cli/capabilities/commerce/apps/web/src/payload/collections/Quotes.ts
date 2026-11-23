import type { CollectionConfig } from "payload";
import { can } from "@fiducial/identity";
import type { Grant, Principal, Resource } from "@fiducial/identity";
import { isOwner, isOwnerOrStaff } from "../access";
import { priceField } from "../fields/price";
import { leadSourceField, utmFields } from "../fields/leadSource";
import { statusTimestampFields, setStatusTimestamp } from "../fields/statusTimestamps";
import { lineItemsJsonField } from "../fields/lineItems";
import { COMMERCE_STATUSES } from "../../generated/commerce";

const STATUS_OPTIONS = COMMERCE_STATUSES.map((s) => ({ label: s, value: s }));

type QuotesUser = { id: string; role: "owner" | "staff" | "agency-admin" };

function grantsFor(user: QuotesUser): Grant[] {
  const principal: Principal = { kind: "user", id: user.id };
  const resource: Resource = { kind: "platform" };
  const roleMap: Record<QuotesUser["role"], Grant["role"]> = {
    owner: "owner",
    staff: "member",
    "agency-admin": "viewer",
  };
  return [{ principal, resource, role: roleMap[user.role] }];
}

export const Quotes: CollectionConfig = {
  slug: "quotes",
  admin: {
    useAsTitle: "id",
    defaultColumns: ["guestName", "status", "createdAt"],
    group: "Commerce",
  },
  access: {
    create: () => true,
    read: isOwnerOrStaff,
    update: isOwnerOrStaff,
    delete: isOwner,
  },
  endpoints: [
    {
      path: "/:id/convert",
      method: "post",
      handler: async (req) => {
        const { payload, routeParams } = req as typeof req & {
          routeParams: { id: string };
        };
        const user = req.user as QuotesUser | null;
        if (!user) {
          return Response.json({ error: "Unauthorized" }, { status: 401 });
        }
        const principal: Principal = { kind: "user", id: user.id };
        const resource: Resource = { kind: "platform" };
        if (!can(principal, "write", resource, grantsFor(user))) {
          return Response.json({ error: "Forbidden" }, { status: 403 });
        }
        const quote = await payload.findByID({ collection: "quotes", id: routeParams.id });
        const now = new Date().toISOString();
        const order = await payload.create({
          collection: "orders",
          data: {
            status: "confirmed",
            confirmedAt: now,
            guestName: quote.guestName,
            guestPhone: quote.guestPhone,
            guestEmail: quote.guestEmail,
            guestCity: quote.guestCity,
            guestAddress: quote.guestAddress,
            lineItemsJson: quote.lineItemsJson,
            total: quote.total,
            leadSource: quote.leadSource,
            utmSource: quote.utmSource,
            utmMedium: quote.utmMedium,
            utmCampaign: quote.utmCampaign,
            note: quote.note,
          },
        });
        await payload.update({
          collection: "quotes",
          id: routeParams.id,
          data: { status: "confirmed", confirmedAt: now },
        });
        return Response.json({ order });
      },
    },
  ],
  fields: [
    {
      name: "status",
      type: "select",
      required: true,
      defaultValue: "quoted",
      options: STATUS_OPTIONS,
      admin: { position: "sidebar" },
    },
    {
      type: "collapsible",
      label: "Guest",
      fields: [
        { name: "guestName", type: "text", required: true },
        { name: "guestPhone", type: "text", required: true },
        { name: "guestEmail", type: "email" },
        { name: "guestCity", type: "text" },
        { name: "guestAddress", type: "textarea" },
      ],
    },
    lineItemsJsonField,
    priceField("total"),
    {
      name: "note",
      type: "textarea",
      localized: true,
    },
    leadSourceField,
    ...utmFields,
    ...statusTimestampFields,
  ],
  hooks: {
    beforeChange: [setStatusTimestamp],
  },
};
