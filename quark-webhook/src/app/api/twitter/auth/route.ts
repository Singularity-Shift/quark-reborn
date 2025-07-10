import { NextRequest, NextResponse } from "next/server";

interface TwitterAuthRequest {
  code: string;
  state: string;
  verifier?: string;
}

interface TwitterAuthResponse {
  success: boolean;
  user?: {
    telegram_username: string;
    twitter_handle: string;
    twitter_id: string;
    follower_count: number;
    qualifies: boolean;
  };
  error?: string;
}

interface TwitterTokenResponse {
  access_token: string;
  refresh_token?: string;
  expires_in: number;
  token_type: string;
  scope: string;
}

interface TwitterUserProfile {
  data: {
    id: string;
    username: string;
    name: string;
    profile_image_url?: string;
    verified?: boolean;
    public_metrics?: {
      followers_count: number;
      following_count: number;
      tweet_count: number;
      listed_count: number;
    };
  };
}

export async function POST(request: NextRequest) {
  try {
    const body: TwitterAuthRequest = await request.json();
    const { code, state, verifier } = body;

    // Validate input
    if (!code || !state) {
      return NextResponse.json(
        { success: false, error: "Missing code or state parameter" },
        { status: 400 }
      );
    }

    if (!verifier) {
      return NextResponse.json(
        { success: false, error: "Missing verifier parameter" },
        { status: 400 }
      );
    }

    // Get Twitter OAuth credentials from environment
    const clientId = process.env.TWITTER_CLIENT_ID;
    const clientSecret = process.env.TWITTER_CLIENT_SECRET;
    const redirectUri = process.env.TWITTER_REDIRECT_URI;

    if (!clientId || !clientSecret || !redirectUri) {
      console.error("Missing Twitter OAuth credentials");
      return NextResponse.json(
        { success: false, error: "Twitter OAuth not configured" },
        { status: 500 }
      );
    }

    // For now, we'll extract telegram username from state (you may need to adjust this)
    // The state should contain encoded user info
    const telegramUsername = decodeURIComponent(state.split('_')[0] || 'unknown');

    // Step 1: Exchange code for access token
    const tokenResponse = await fetch("https://api.twitter.com/2/oauth2/token", {
      method: "POST",
      headers: {
        "Content-Type": "application/x-www-form-urlencoded",
        "Authorization": `Basic ${Buffer.from(`${clientId}:${clientSecret}`).toString('base64')}`,
      },
             body: new URLSearchParams({
         grant_type: "authorization_code",
         code: code,
         redirect_uri: redirectUri,
         code_verifier: verifier,
       }),
    });

    if (!tokenResponse.ok) {
      const errorText = await tokenResponse.text();
      console.error("Twitter token exchange failed:", errorText);
      return NextResponse.json(
        { success: false, error: "Failed to exchange code for token" },
        { status: 400 }
      );
    }

    const tokenData: TwitterTokenResponse = await tokenResponse.json();

    // Step 2: Get user profile using access token
    const profileResponse = await fetch("https://api.twitter.com/2/users/me?user.fields=id,username,name,profile_image_url,verified,public_metrics", {
      method: "GET",
      headers: {
        "Authorization": `Bearer ${tokenData.access_token}`,
      },
    });

    if (!profileResponse.ok) {
      const errorText = await profileResponse.text();
      console.error("Twitter profile fetch failed:", errorText);
      return NextResponse.json(
        { success: false, error: "Failed to fetch user profile" },
        { status: 400 }
      );
    }

    const profileData: TwitterUserProfile = await profileResponse.json();
    const user = profileData.data;

    // Check qualification criteria (you can adjust these)
    const followerCount = user.public_metrics?.followers_count || 0;
    const hasProfilePic = !!user.profile_image_url;
    const isVerified = user.verified || false;
    
    // Simple qualification logic - adjust as needed
    const qualifies = followerCount >= 100 || isVerified || hasProfilePic;

    // TODO: Store the user data in your database here
    console.log("Twitter user authenticated:", {
      telegram_username: telegramUsername,
      twitter_handle: user.username,
      twitter_id: user.id,
      follower_count: followerCount,
      qualifies
    });

    return NextResponse.json({
      success: true,
      user: {
        telegram_username: telegramUsername,
        twitter_handle: user.username,
        twitter_id: user.id,
        follower_count: followerCount,
        qualifies,
      },
    });

  } catch (error) {
    console.error("Twitter auth API error:", error);
    return NextResponse.json(
      { success: false, error: "Internal server error" },
      { status: 500 }
    );
  }
}

export async function GET() {
  return NextResponse.json(
    { error: "Method not allowed" },
    { status: 405 }
  );
} 