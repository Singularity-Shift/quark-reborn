import React from 'react';
import { Sparkles, Lock } from 'lucide-react';

export const SubscriptionUpgradeContainer = () => {
  return (
    <div className="w-full max-w-[400px] h-[600px] bg-white bg-opacity-90 p-6 lg:p-10 rounded-xl shadow-lg border border-gray-300 flex flex-col">
      <div className="text-center">
        <div className="flex items-center justify-center gap-2">
          <Sparkles className="w-6 h-6 text-yellow-500" />
          <h2 className="mt-6 text-2xl lg:text-3xl font-extrabold text-gray-900">
            Premium Features
          </h2>
          <Sparkles className="w-6 h-6 text-yellow-500" />
        </div>
        <p className="mt-2 text-sm text-gray-600">Unlock the full power of SShift GPT</p>
      </div>
      <div className="flex-grow flex flex-col items-center justify-center space-y-6">
        <div className="flex flex-col items-center space-y-4">
          <div className="w-16 h-16 rounded-full bg-gradient-to-r from-purple-400 via-pink-500 to-red-500 flex items-center justify-center">
            <Lock className="w-8 h-8 text-white" />
          </div>
          <p className="text-lg text-gray-800 text-center font-semibold">
            Elevate your experience with a subscription
          </p>
        </div>
        <div className="space-y-3 text-sm text-gray-700">
          <div className="flex items-center gap-2">
            <Sparkles className="w-4 h-4 text-yellow-500" />
            <p><span className="font-bold">Access All Models</span> - Including cutting-edge GPT-4o, O4-mini and future SOTA models</p>
          </div>
          <div className="flex items-center gap-2">
            <Sparkles className="w-4 h-4 text-yellow-500" />
            <p><span className="font-bold">Premium Tools</span> - Image generation, sound effects, handle finder & more</p>
          </div>
          <div className="flex items-center gap-2">
            <Sparkles className="w-4 h-4 text-yellow-500" />
            <p><span className="font-bold">Higher Rate Limits</span> - Much higher rate limits</p>
          </div>
          <div className="flex items-center gap-2">
            <Sparkles className="w-4 h-4 text-yellow-500" />
            <p><span className="font-bold">Advanced Features</span> - Free use of non PAYG Super-Apps</p>
          </div>
        </div>
      </div>
    </div>
  );
};
