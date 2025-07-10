import { NextRequest, NextResponse } from "next/server";

export const runtime = "nodejs";

interface TwitterOAuthUrlRequest {
  userId: string;
  state: string;
  challenge: string;
}

export async function POST(request: NextRequest) {
  try {
    const body: TwitterOAuthUrlRequest = await request.json();
    const { userId, state, challenge } = body;

    // Validate input
    if (!userId || !state || !challenge) {
      return NextResponse.json(
        { success: false, error: "Missing required parameters" },
        { status: 400 }
      );
    }

    // Get Twitter OAuth credentials from environment (server-side only)
    const clientId = process.env.TWITTER_CLIENT_ID || process.env.NEXT_PUBLIC_TWITTER_CLIENT_ID;
    const redirectUri = process.env.TWITTER_REDIRECT_URI || process.env.NEXT_PUBLIC_TWITTER_REDIRECT_URI;

    if (!clientId || !redirectUri) {
      console.error("Twitter OAuth environment variables not configured");
      return NextResponse.json(
        { success: false, error: "Twitter OAuth not configured" },
        { status: 500 }
      );
    }

    // Build Twitter OAuth URL
    const authUrl = `https://twitter.com/i/oauth2/authorize?response_type=code&client_id=${clientId}&redirect_uri=${encodeURIComponent(redirectUri)}&scope=tweet.read%20users.read&state=${encodeURIComponent(state)}&code_challenge=${encodeURIComponent(challenge)}&code_challenge_method=S256`;

    return NextResponse.json({
      success: true,
      authUrl,
    });

  } catch (error) {
    console.error("Twitter OAuth URL generation error:", error);
    return NextResponse.json(
      { success: false, error: "Internal server error" },
      { status: 500 }
    );
  }
} 