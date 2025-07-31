import { AgentRuntime } from "move-agent-kit-fullstack";
import { twMerge } from "tailwind-merge";
import { type ClassValue, clsx } from "clsx";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export const transformCoverUrl = (url: string) => {
  if (!url) return null;

  // Handle IPFS URLs
  if (url.startsWith("ipfs://")) {
    return `https://ipfs.io/ipfs/${url.replace("ipfs://", "")}`;
  }

  // Handle Aptos Names API URLs
  if (url.includes("aptos-names-api")) {
    // The URL is already in the correct format for direct access
    return url;
  }

  return url;
};

export const getUserSubscription = async (
  agent: AgentRuntime,
  account: string
) => {
  let hasSubscription = false;

  const collectors = (await agent.aptos.view({
    payload: {
      function: `${process.env.NEXT_PUBLIC_SSHIFT_MODULE_ADDRESS}::fees::get_collectors`,
      typeArguments: [],
      functionArguments: [],
    },
  })) as any;

  const hasSubscriptionActive = await agent.aptos.view({
    payload: {
      function: `${process.env.NEXT_PUBLIC_SSHIFT_MODULE_ADDRESS}::subscription::has_subscription_active`,
      typeArguments: [],
      functionArguments: [account],
    },
  });

  if (hasSubscriptionActive[0]) {
    const plan = await agent.aptos.view({
      payload: {
        function: `${process.env.NEXT_PUBLIC_SSHIFT_MODULE_ADDRESS}::subscription::get_plan`,
        typeArguments: [],
        functionArguments: [account],
      },
    });

    hasSubscription = !plan[3];
  }

  return (
    collectors?.[0]?.some(
      (c: string) => c.toLowerCase() === account.toLowerCase()
    ) || hasSubscription
  );
};

export const actions = [
  "abusefiltercheckmatch",
  "abusefilterchecksyntax",
  "abusefilterevalexpression",
  "abusefilterunblockautopromote",
  "abuselogprivatedetails",
  "acquiretempusername",
  "antispoof",
  "block",
  "centralauthtoken",
  "centralnoticecdncacheupdatebanner",
  "centralnoticechoicedata",
  "centralnoticequerycampaign",
  "changeauthenticationdata",
  "changecontentmodel",
  "checktoken",
  "cirrus-config-dump",
  "cirrus-mapping-dump",
  "cirrus-profiles-dump",
  "cirrus-settings-dump",
  "clearhasmsg",
  "clientlogin",
  "communityconfigurationedit",
  "compare",
  "createaccount",
  "createlocalaccount",
  "cxdelete",
  "cxsuggestionlist",
  "cxtoken",
  "delete",
  "deleteglobalaccount",
  "discussiontoolsedit",
  "discussiontoolsfindcomment",
  "discussiontoolsgetsubscriptions",
  "discussiontoolssubscribe",
  "discussiontoolsthank",
  "echocreateevent",
  "echomarkread",
  "echomarkseen",
  "echomute",
  "edit",
  "editmassmessagelist",
  "emailuser",
  "expandtemplates",
  "featuredfeed",
  "feedcontributions",
  "feedrecentchanges",
  "feedwatchlist",
  "filerevert",
  "flagconfig",
  "globalblock",
  "globalpreferenceoverrides",
  "globalpreferences",
  "globaluserrights",
  "growthmanagementorlist",
  "growthmentordashboardupdatedata",
  "growthsetmenteestatus",
  "growthsetmentor",
  "growthstarmentee",
  "help",
  "homepagequestionstore",
  "imagerotate",
  "import",
  "jsonconfig",
  "languagesearch",
  "linkaccount",
  "login",
  "logout",
  "managetags",
  "massmessage",
  "mergehistory",
  "move",
  "opensearch",
  "options",
  "pagetriageaction",
  "pagetriagelist",
  "pagetriagestats",
  "pagetriagetagcopyvio",
  "pagetriagetagging",
  "paraminfo",
  "parse",
  "patrol",
  "protect",
  "purge",
  "query",
  "removeauthenticationdata",
  "resetpassword",
  "review",
  "revisiondelete",
  "rollback",
  "rsd",
  "setglobalaccountstatus",
  "setnotificationtimestamp",
  "setpagelanguage",
  "shortenurl",
  "sitematrix",
  "spamblacklist",
  "stabilize",
  "streamconfigs",
  "strikevote",
  "sxdelete",
  "tag",
  "templatedata",
  "thank",
  "titleblacklist",
  "torblock",
  "transcodereset",
  "unblock",
  "undelete",
  "unlinkaccount",
  "upload",
  "userrights",
  "validatepassword",
  "watch",
  "webapp-manifest",
  "webauthn",
  "wikilove",
  "bouncehandler",
  "categorytree",
  "cirrus-check-sanity",
  "collection",
  "cspreport",
  "cxcheckunreviewed",
  "cxpublish",
  "cxpublishsection",
  "cxsave",
  "cxsplit",
  "discussiontoolscompare",
  "discussiontoolspageinfo",
  "discussiontoolspreview",
  "echopushsubscriptions",
  "editcheckreferenceurl",
  "fancycaptchareload",
  "growthinvalidateimagerecommendation",
  "growthinvalidatepersonalizedpraisesuggestion",
  "helppanelquestionposter",
  "jsondata",
  "oathvalidate",
  "parser-migration",
  "readinglists",
  "sanitize-mapdata",
  "scribunto-console",
  "securepollauth",
  "stashedit",
  "sxsave",
  "timedtext",
  "ulslocalization",
  "ulssetlang",
  "visualeditor",
  "visualeditoredit",
  "wikimediaeventsblockededit",
];
