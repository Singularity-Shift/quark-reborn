'use client';

import { useEffect, useState, useRef } from 'react';
import { useSearchParams } from 'next/navigation';

export default function TwitterLoginPage() {
  const [status, setStatus] = useState<'loading' | 'redirecting' | 'error'>('loading');
  const [error, setError] = useState<string>('');
  const searchParams = useSearchParams();
  const hasRun = useRef(false);

  // Extract search parameters once
  const userId = searchParams.get('userId');
  const state = searchParams.get('state');
  const challenge = searchParams.get('challenge');
  const verifier = searchParams.get('verifier');

  useEffect(() => {
    if (hasRun.current) return;
    hasRun.current = true;

    if (!userId || !state || !challenge || !verifier) {
      setStatus('error');
      setError('Missing required parameters');
      return;
    }

    // Store verifier for the auth callback
    sessionStorage.setItem('oauth_verifier', verifier);
    sessionStorage.setItem('oauth_state', state);

    // Redirect to Twitter OAuth
    const twitterAuthUrl = new URL('https://twitter.com/i/oauth2/authorize');
    twitterAuthUrl.searchParams.set('response_type', 'code');
    twitterAuthUrl.searchParams.set('client_id', process.env.NEXT_PUBLIC_TWITTER_CLIENT_ID || '');
    twitterAuthUrl.searchParams.set('redirect_uri', process.env.NEXT_PUBLIC_TWITTER_REDIRECT_URI || '');
    twitterAuthUrl.searchParams.set('scope', 'tweet.read users.read offline.access');
    twitterAuthUrl.searchParams.set('state', state);
    twitterAuthUrl.searchParams.set('code_challenge', challenge);
    twitterAuthUrl.searchParams.set('code_challenge_method', 'S256');

    setStatus('redirecting');
    window.location.href = twitterAuthUrl.toString();
  }, [userId, state, challenge, verifier]);

  if (status === 'loading') {
    return (
      <div className="min-h-screen bg-gradient-to-br from-blue-50 to-indigo-100 flex items-center justify-center p-4">
        <div className="bg-white rounded-xl shadow-lg p-8 max-w-md w-full text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mx-auto mb-4"></div>
          <h2 className="text-xl font-semibold text-gray-900 mb-2">Preparing Twitter Login</h2>
          <p className="text-gray-600">Setting up your authentication...</p>
        </div>
      </div>
    );
  }

  if (status === 'redirecting') {
    return (
      <div className="min-h-screen bg-gradient-to-br from-blue-50 to-indigo-100 flex items-center justify-center p-4">
        <div className="bg-white rounded-xl shadow-lg p-8 max-w-md w-full text-center">
          <div className="text-blue-600 mb-4">
            <svg className="w-12 h-12 mx-auto" fill="currentColor" viewBox="0 0 24 24">
              <path d="M23.953 4.57a10 10 0 01-2.825.775 4.958 4.958 0 002.163-2.723c-.951.555-2.005.959-3.127 1.184a4.92 4.92 0 00-8.384 4.482C7.69 8.095 4.067 6.13 1.64 3.162a4.822 4.822 0 00-.666 2.475c0 1.71.87 3.213 2.188 4.096a4.904 4.904 0 01-2.228-.616v.06a4.923 4.923 0 003.946 4.827 4.996 4.996 0 01-2.212.085 4.936 4.936 0 004.604 3.417 9.867 9.867 0 01-6.102 2.105c-.39 0-.779-.023-1.17-.067a13.995 13.995 0 007.557 2.209c9.053 0 13.998-7.496 13.998-13.985 0-.21 0-.42-.015-.63A9.935 9.935 0 0024 4.59z"/>
            </svg>
          </div>
          <h2 className="text-xl font-semibold text-gray-900 mb-2">Redirecting to Twitter</h2>
          <p className="text-gray-600">You will be redirected to Twitter to complete authentication...</p>
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
        <h2 className="text-xl font-semibold text-gray-900 mb-2">Authentication Error</h2>
        <p className="text-gray-600 mb-4">{error}</p>
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