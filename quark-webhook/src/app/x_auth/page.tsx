'use client';

import { useEffect, useState, useRef } from 'react';
import { useSearchParams } from 'next/navigation';

// Telegram WebApp types
declare global {
  interface Window {
    Telegram?: {
      WebApp: {
        sendData: (data: string) => void;
        close: () => void;
      };
    };
  }
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

export default function TwitterCallbackPage() {
  const [status, setStatus] = useState<'loading' | 'success' | 'error'>('loading');
  const [result, setResult] = useState<TwitterAuthResponse | null>(null);
  const searchParams = useSearchParams();
  const hasRun = useRef(false);
  
  // Extract search parameters once
  const code = searchParams.get('code');
  const state = searchParams.get('state');
  const error = searchParams.get('error');

  useEffect(() => {
    const handleCallback = async () => {
      if (hasRun.current) return;
      hasRun.current = true;
      try {
        if (error) {
          throw new Error(`Twitter OAuth error: ${error}`);
        }

        if (!code || !state) {
          throw new Error('Missing code or state from Twitter callback');
        }

        // Get verifier from sessionStorage
        const verifier = sessionStorage.getItem('oauth_verifier');
        const storedState = sessionStorage.getItem('oauth_state');

        if (!verifier) {
          throw new Error('OAuth verifier not found. Please try again.');
        }

        if (storedState !== state) {
          throw new Error('OAuth state mismatch. Please try again.');
        }

        // Exchange code for token
        const response = await fetch('/api/twitter/auth', {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: JSON.stringify({
            code,
            state,
            verifier,
          }),
        });

        const authResult: TwitterAuthResponse = await response.json();

        if (!response.ok) {
          throw new Error(authResult.error || 'Authentication failed');
        }

        setResult(authResult);
        setStatus(authResult.success ? 'success' : 'error');

        // Clean up sessionStorage
        sessionStorage.removeItem('oauth_verifier');
        sessionStorage.removeItem('oauth_state');

        // Send result back to Telegram Web App
        if (window.Telegram?.WebApp) {
          const payload = authResult.success 
            ? {
                type: 'twitter_auth_success',
                user: authResult.user,
              }
            : {
                type: 'twitter_auth_failure', 
                error: authResult.error || 'Authentication failed',
              };

          window.Telegram.WebApp.sendData(JSON.stringify(payload));
          window.Telegram.WebApp.close();
        }

      } catch (error) {
        console.error('Twitter auth callback error:', error);
        const errorMessage = error instanceof Error ? error.message : 'Unknown error occurred';
        
        setResult({ success: false, error: errorMessage });
        setStatus('error');

        // Send error back to Telegram Web App
        if (window.Telegram?.WebApp) {
          window.Telegram.WebApp.sendData(JSON.stringify({
            type: 'twitter_auth_failure',
            error: errorMessage,
          }));
          window.Telegram.WebApp.close();
        }
      }
    };

    handleCallback();
  }, [code, state, error]);

  if (status === 'loading') {
    return (
      <div className="min-h-screen bg-gradient-to-br from-blue-50 to-indigo-100 flex items-center justify-center p-4">
        <div className="bg-white rounded-xl shadow-lg p-8 max-w-md w-full text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto mb-4"></div>
          <h2 className="text-xl font-semibold text-gray-900 mb-2">Processing Authentication</h2>
          <p className="text-gray-600">Verifying your Twitter account...</p>
        </div>
      </div>
    );
  }

  if (status === 'success' && result?.user) {
    return (
      <div className="min-h-screen bg-gradient-to-br from-green-50 to-emerald-100 flex items-center justify-center p-4">
        <div className="bg-white rounded-xl shadow-lg p-8 max-w-md w-full text-center">
          <div className="text-green-600 mb-4">
            <svg className="w-12 h-12 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" />
            </svg>
          </div>
          <h2 className="text-xl font-semibold text-gray-900 mb-2">
            {result.user.qualifies ? 'Qualified!' : 'Connected!'}
          </h2>
          <div className="text-left bg-gray-50 rounded-lg p-4 mb-4">
            <p className="text-sm text-gray-600 mb-1">
              <span className="font-medium">Handle:</span> @{result.user.twitter_handle}
            </p>
            <p className="text-sm text-gray-600 mb-1">
              <span className="font-medium">Followers:</span> {result.user.follower_count.toLocaleString()}
            </p>
            <p className="text-sm text-gray-600">
              <span className="font-medium">Status:</span>{' '}
              <span className={result.user.qualifies ? 'text-green-600' : 'text-orange-600'}>
                {result.user.qualifies ? 'Qualified for raids' : 'Not qualified yet'}
              </span>
            </p>
          </div>
          {!result.user.qualifies && (
            <p className="text-xs text-gray-500 mb-4">
              To qualify: Get 100+ followers, add profile pic, or get verified
            </p>
          )}
          <p className="text-gray-600 text-sm">You can close this window</p>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gradient-to-br from-red-50 to-red-100 flex items-center justify-center p-4">
      <div className="bg-white rounded-xl shadow-lg p-8 max-w-md w-full text-center">
        <div className="text-red-600 mb-4">
          <svg className="w-12 h-12 mx-auto" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
        </div>
        <h2 className="text-xl font-semibold text-gray-900 mb-2">Authentication Failed</h2>
        <p className="text-gray-600 mb-4">{result?.error || 'Something went wrong'}</p>
        <button 
          onClick={() => window.close()}
          className="bg-red-600 text-white px-4 py-2 rounded-lg hover:bg-red-700 transition-colors"
        >
          Close
        </button>
      </div>
    </div>
  );
} 