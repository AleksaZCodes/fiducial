import type { CollectionConfig } from "payload";
import type { PayloadRole } from "../../generated/payload";
import { isAuthenticated, isOwner, isOwnerOrStaff } from "../access";

const ROLES: PayloadRole[] = ["owner", "staff", "agency-admin"];

export const Users: CollectionConfig = {
  slug: "users",
  auth: true,
  admin: {
    useAsTitle: "email",
    defaultColumns: ["email", "role"],
  },
  access: {
    create: isOwner,
    read: isOwnerOrStaff,
    update: isOwnerOrStaff,
    delete: isOwner,
  },
  fields: [
    {
      name: "role",
      type: "select",
      required: true,
      defaultValue: "staff",
      options: ROLES.map((r) => ({ label: r, value: r })),
    },
    {
      name: "name",
      type: "text",
    },
  ],
};
