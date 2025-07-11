import type { NextConfig } from "next";
import createNextIntlPlugin from "next-intl/plugin";

const withNextIntl = createNextIntlPlugin("./src/core/i18n/i18n.ts");

const nextConfig: NextConfig = {
  env: {
    ACCOUNT_SEED: process.env.ACCOUNT_SEED,
    APTOS_NODE_URL: process.env.APTOS_NODE_URL,
    APTOS_INDEXER: process.env.APTOS_INDEXER,
  },
};

export default withNextIntl(nextConfig);
