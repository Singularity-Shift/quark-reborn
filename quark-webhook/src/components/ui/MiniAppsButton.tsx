import React from 'react';
import { Grid3X3 } from 'lucide-react';

interface MiniAppsButtonProps {
  onToggleMiniApps: () => void;
}

export const MiniAppsButton: React.FC<MiniAppsButtonProps> = ({
  onToggleMiniApps,
}) => {
  return (
    <button
      onClick={onToggleMiniApps}
      className="flex items-center justify-center w-10 h-10 bg-white rounded-full shadow-md hover:shadow-lg transition-shadow border border-gray-200 focus:outline-none focus:ring-2 focus:ring-blue-300"
      aria-label="Open Mini Apps"
      title="Mini Apps"
    >
      <Grid3X3 className="h-5 w-5 text-blue-500" />
    </button>
  );
}; 