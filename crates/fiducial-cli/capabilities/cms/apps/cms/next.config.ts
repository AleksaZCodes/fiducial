import type { NextConfig } from "next";

// Only used when `[cms] editor = "keystatic"`; Tina serves its own admin.
const config: NextConfig = { reactStrictMode: true, poweredByHeader: false };

export default config;
