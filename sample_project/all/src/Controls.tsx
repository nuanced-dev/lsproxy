import React from 'react';

interface ControlsProps {
    onFindPath: () => void;
    onReset: () => void;
    isAnimating: boolean;
}

export const Controls: React.FC<ControlsProps> = ({
    onFindPath,
    onReset,
    isAnimating
}) => {
    return (
        <div className="mb-4 space-x-2">
            <button
                onClick={onFindPath}
                className="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600"
                disabled={isAnimating}
            >
                {isAnimating ? 'Finding Path...' : 'Find Path'}
            </button>
            <button
                onClick={onReset}
                className="px-4 py-2 bg-gray-500 text-white rounded hover:bg-gray-600"
                disabled={isAnimating}
            >
                Reset
            </button>
        </div>
    );
};
