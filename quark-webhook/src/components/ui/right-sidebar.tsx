import React from 'react';
import { Button } from './button';
import { ScrollArea } from './scrollarea';
import { X } from 'lucide-react';
import Link from 'next/link';

interface RightSidebarProps {
  isOpen?: boolean;
  onClose?: () => void;
}

interface MiniAppProps {
  title: string;
  icon: React.ReactNode;
  children: React.ReactNode;
}

const MiniApp: React.FC<MiniAppProps> = ({ title, icon, children }) => {
  return (
    <div className="bg-white rounded-lg border border-gray-200 p-4 shadow-sm hover:shadow-md transition-shadow">
      <div className="flex items-center gap-2 mb-3">
        {icon}
        <h3 className="font-medium text-sm">{title}</h3>
      </div>
      {children}
    </div>
  );
};

export const RightSidebar: React.FC<RightSidebarProps> = ({
  isOpen = false,
  onClose,
}) => {
  return (
    <>
      {/* Backdrop overlay for mobile */}
      {isOpen && (
        <div 
          className="fixed inset-0 bg-black/40 backdrop-blur-sm z-40" 
          onClick={onClose}
        />
      )}
      <div className={`
        fixed top-0 right-0 h-[100dvh] w-80 border-l border-border
        bg-white
        transform transition-transform duration-300 ease-in-out z-50
        ${isOpen ? 'translate-x-0' : 'translate-x-full'}
      `}>
        <div className="flex flex-col h-full">
          <div className="p-4 border-b border-border h-[73px] flex items-center justify-between">
            <div className="flex items-center gap-2">
              <div className="grid grid-cols-2 gap-0.5 w-4 h-4">
                <div className="bg-blue-500 rounded-sm"></div>
                <div className="bg-blue-500 rounded-sm"></div>
                <div className="bg-blue-500 rounded-sm"></div>
                <div className="bg-blue-500 rounded-sm"></div>
              </div>
              <h2 className="text-lg font-semibold">Super-Apps</h2>
            </div>
            <div className="flex items-center gap-2">
              <Button
                variant="ghost"
                size="sm"
                onClick={onClose}
                className="flex items-center justify-center h-9 w-9 rounded-full hover:bg-gray-100"
                aria-label="Close Mini Apps"
                title="Close Mini Apps"
              >
                <X className="h-4 w-4" />
              </Button>
            </div>
          </div>
          <ScrollArea className="flex-1">
            <div className="p-4 space-y-6">
              {/* Subscription Apps Section */}
              <div className="space-y-2">
                <h3 className="text-sm font-semibold text-gray-700">Free with subscription</h3>
                <Link href={process.env.NEXT_LEDGER_APP_URL || 'https://ledgerapp.fun'}>
                  <Button className="w-full justify-start gap-2 p-4">
                    <span className="text-xl">📒</span>
                    <div className="flex flex-col items-start">
                      <span className="font-medium">Ledger App</span>
                      <span className="text-xs text-gray-500">Infinite DIY NFT collection</span>
                    </div>
                  </Button>
                </Link>
              </div>

              {/* Pay-per-use Apps Section */}
              <div className="space-y-2">
                <h3 className="text-sm font-semibold text-gray-700">Pay-per-use</h3>
                <div className="bg-white rounded-lg border border-gray-200 p-4 shadow-sm text-center text-gray-500 italic">
                  Apps in development
                </div>
              </div>

              {/* Free Apps Section */}
              <div className="space-y-2">
                <h3 className="text-sm font-semibold text-gray-700">Free for all</h3>
                <div className="bg-white rounded-lg border border-gray-200 p-4 shadow-sm text-center text-gray-500 italic">
                  Apps in development
                </div>
              </div>
            </div>
          </ScrollArea>
        </div>
      </div>
    </>
  );
}; 