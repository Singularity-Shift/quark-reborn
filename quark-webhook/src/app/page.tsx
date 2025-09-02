"use client";

import { Section, Cell, Image, List } from "@telegram-apps/telegram-ui";
import { useTranslations } from "next-intl";

import { Link } from "@/components/Link/Link";
import { Page } from "@/components/Page";

export default function Home() {
  const t = useTranslations("i18n");

  return (
    <Page back={false}>
      <List>
        <Section footer="Login to your Nova account">
          <Link href="/login">
            <Cell>Login</Cell>
          </Link>
        </Section>
      </List>
    </Page>
  );
}
