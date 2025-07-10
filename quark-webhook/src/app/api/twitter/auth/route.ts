import { NextRequest, NextResponse } from "next/server";

interface TwitterAuthRequest {
  code: string;
  state: string;
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

export async function POST(request: NextRequest) {
  try {
    const body: TwitterAuthRequest = await request.json();
    const { code, state } = body;

    // Validate input
    if (!code || !state) {
      return NextResponse.json(
        { success: false, error: "Missing code or state parameter" },
        { status: 400 }
      );
    }

    // Get backend URL from environment
    const backendUrl = process.env.BACKEND_URL || "http://quark-server:3200";
    
    // Forward the request to the quark_server backend
    const backendResponse = await fetch(`${backendUrl}/api/twitter/oauth/callback`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        code,
        state,
      }),
    });

    if (!backendResponse.ok) {
      const errorText = await backendResponse.text();
      console.error("Backend error:", errorText);
      return NextResponse.json(
        { success: false, error: "Backend authentication failed" },
        { status: backendResponse.status }
      );
    }

    const result: TwitterAuthResponse = await backendResponse.json();
    return NextResponse.json(result);

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