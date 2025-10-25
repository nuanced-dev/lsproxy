import React from 'react';
import { Node } from './node';
import { Grid } from './types';

interface GridDisplayProps {
    maze: Grid;
    start: { x: number; y: number };
    end: { x: number; y: number };
    path: Node[] | null;
    currentStep: number;
    isAnimating: boolean;
    onCellClick: (x: number, y: number) => void;
}

export const GridDisplay: React.FC<GridDisplayProps> = ({
    maze,
    start,
    end,
    path,
    currentStep,
    onCellClick
}) => {
    const getCellColor = (x: number, y: number): string => {
        if (x === start.x && y === start.y) return 'bg-green-500';
        if (x === end.x && y === end.y) return 'bg-red-500';
        if (path?.slice(0, currentStep + 1).some(node => node.x === x && node.y === y)) {
            return 'bg-blue-500';
        }
        if (maze[y][x] === 100) return 'bg-gray-500';
        if (maze[y][x] === -1) return 'bg-black';
        return 'bg-white';
    };

    return (
        <div className="inline-block border border-gray-200">
            {maze.map((row, y) => (
                <div key={y} className="flex">
                    {row.map((_, x) => (
                        <div
                            key={`${x}-${y}`}
                            className={`w-8 h-8 border border-gray-200 ${getCellColor(x, y)} 
                                transition-colors duration-300 cursor-pointer`}
                            onClick={() => onCellClick(x, y)}
                        />
                    ))}
                </div>
            ))}
        </div>
    );
};
