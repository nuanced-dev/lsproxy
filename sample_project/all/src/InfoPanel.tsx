import React from 'react';
import { Node } from './node';

interface InfoPanelProps {
    path: Node[] | null;
}

export const InfoPanel: React.FC<InfoPanelProps> = ({ path }) => {
    return (
        <div className="mt-4">
            <div className="text-sm text-gray-600">
                Click cells to toggle walls. Green = Start, Red = End, Blue = Path
            </div>
            {path && (
                <div className="mt-2 text-sm">
                    Path length: {path.length} steps
                    <br />
                    Total cost: {path[path.length - 1].g.toFixed(2)}
                </div>
            )}
        </div>
    );
};
