import type { CollectionConfig } from "payload";
import { can } from "@fiducial/identity";
import type { Grant, Principal, Resource } from "@fiducial/identity";
import { isOwner, isOwnerOrStaff } from "../access";
import { priceField } from "../fields/price";
import { leadSourceField, utmFields } from "../fields/leadSource";
import { statusTimestampFields, setStatusTimestamp } from "../fields/statusTimestamps";
import { lineItemsJsonField } from "../fields/lineItems";
import { COMMERCE_STATUSES } from "../../generated/commerce";
import { notifyOnCreateOrder } from "../hooks/notifyOnCreate";

const STATUS_OPTIONS = COMMERCE_STATUSES.map((s) => ({ label: s, value: s }));

type OrdersUser = { id: string; role: "owner" | "staff" | "agency-admin" };

function grantsFor(user: OrdersUser): Grant[] {
  const principal: Principal = { kind: "user", id: user.id };
  const resource: Resource = { kind: "platform" };
  const roleMap: Record<OrdersUser["role"], Grant["role"]> = {
    owner: "owner",
    staff: "member",
    "agency-admin": "viewer",
  };
  return [{ principal, resource, role: roleMap[user.role] }];
}

export const Orders: CollectionConfig = {
  slug: "orders",
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
      path: "/:id/mark-paid",
      method: "post",
      handler: async (req) => {
        const { payload, routeParams } = req as typeof req & {
          routeParams: { id: string };
        };
        const user = req.user as OrdersUser | null;
        if (!user) {
          return Response.json({ error: "Unauthorized" }, { status: 401 });
        }
        const principal: Principal = { kind: "user", id: user.id };
        const resource: Resource = { kind: "platform" };
        if (!can(principal, "write", resource, grantsFor(user))) {
          return Response.json({ error: "Forbidden" }, { status: 403 });
        }
        const now = new Date().toISOString();
        const updated = await payload.update({
          collection: "orders",
          id: routeParams.id,
          data: { status: "paid", paidAt: now },
        });
        return Response.json(updated);
      },
    },
  ],
  fields: [
    {
      name: "status",
      type: "select",
      required: true,
      defaultValue: "new",
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
      name: "discountMinor",
      type: "number",
      defaultValue: 0,
      admin: {
        description: "Phone-agreed discount in minor units. Not computed automatically.",
      },
    },
    { name: "note", type: "textarea", localized: true },
    leadSourceField,
    ...utmFields,
    ...statusTimestampFields,
  ],
  hooks: {
    beforeChange: [setStatusTimestamp],
    afterChange: [notifyOnCreateOrder],
  },
};
